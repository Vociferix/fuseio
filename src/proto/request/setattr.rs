use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileFlag, FileHandle, FileTime, Gid, Mode, Uid};
use crate::{Error, Result, buf::Buf};

use std::time::SystemTime;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct ValidFlags: u32 {
        const MODE = 1 << 0;
        const UID = 1 << 1;
        const GID = 1 << 2;
        const SIZE = 1 << 3;
        const ATIME = 1 << 4;
        const MTIME = 1 << 5;
        const FH = 1 << 6;
        const ATIME_NOW = 1 << 7;
        const MTIME_NOW = 1 << 8;
        // Bit 9 is LOCKOWNER, which is only used for mandatory locking during a
        // truncate and is ignored here, as libfuse does.
        const CTIME = 1 << 10;
        // One flag covers both bits: "clear setuid, and setgid if the file is
        // group-executable". libfuse's `FUSE_SET_ATTR_KILL_SUID`/`_SGID` name
        // bits 11 and 12, but those mirror the Linux VFS's internal `ATTR_*`
        // numbering; on the wire bit 12 is unused.
        const KILL_SUIDGID = 1 << 11;

        // macos only flags
        #[cfg(target_os = "macos")]
        const CRTIME = 1 << 28;
        #[cfg(target_os = "macos")]
        const CHGTIME = 1 << 29;
        #[cfg(target_os = "macos")]
        const BKUPTIME = 1 << 30;
        #[cfg(target_os = "macos")]
        const FLAGS = 1 << 31;
    }
}

#[derive(Debug)]
pub struct SetAttr {
    ino: Ino,
    valid: ValidFlags,
    fh: FileHandle,
    size: u64,
    atime: FileTime,
    mtime: FileTime,
    ctime: SystemTime,
    mode: Mode,
    uid: Uid,
    gid: Gid,
    #[cfg(target_os = "macos")]
    bkuptime: SystemTime,
    #[cfg(target_os = "macos")]
    crtime: SystemTime,
    #[cfg(target_os = "macos")]
    flags: FileFlag,
}

#[repr(C)]
struct Raw {
    valid: u32,
    _unused0: u32,
    fh: u64,
    size: u64,
    lock_owner: u64, // apparently ignored by libfuse
    atime: u64,
    mtime: u64,
    ctime: u64, // unused on macos
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32, // unused on macos
    mode: u32,
    _unused1: u32,
    uid: u32,
    gid: u32,
    _unused2: u32,

    #[cfg(target_os = "macos")]
    bkuptime: u64,
    #[cfg(target_os = "macos")]
    chgtime: u64,
    #[cfg(target_os = "macos")]
    crtime: u64,
    #[cfg(target_os = "macos")]
    bkuptimensec: u32,
    #[cfg(target_os = "macos")]
    chgtimensec: u32,
    #[cfg(target_os = "macos")]
    crtimensec: u32,
    #[cfg(target_os = "macos")]
    flags: u32,
}

impl SetAttr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.valid.contains(ValidFlags::FH).then_some(self.fh)
    }

    pub fn size(&self) -> Option<u64> {
        self.valid.contains(ValidFlags::SIZE).then_some(self.size)
    }

    pub fn atime(&self) -> Option<FileTime> {
        (!(self.valid & (ValidFlags::ATIME | ValidFlags::ATIME_NOW)).is_empty())
            .then_some(self.atime)
    }

    pub fn mtime(&self) -> Option<FileTime> {
        (!(self.valid & (ValidFlags::MTIME | ValidFlags::MTIME_NOW)).is_empty())
            .then_some(self.mtime)
    }

    /// The change time to set.
    ///
    /// macOS sends this as its own Darwin flag, since its protocol version
    /// predates the portable one; both arrive here.
    pub fn ctime(&self) -> Option<SystemTime> {
        #[cfg(target_os = "macos")]
        let valid = ValidFlags::CTIME | ValidFlags::CHGTIME;
        #[cfg(not(target_os = "macos"))]
        let valid = ValidFlags::CTIME;

        (!(self.valid & valid).is_empty()).then_some(self.ctime)
    }

    pub fn bkuptime(&self) -> Option<SystemTime> {
        #[cfg(target_os = "macos")]
        {
            self.valid
                .contains(ValidFlags::BKUPTIME)
                .then_some(self.bkuptime)
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub fn crtime(&self) -> Option<SystemTime> {
        #[cfg(target_os = "macos")]
        {
            self.valid
                .contains(ValidFlags::CRTIME)
                .then_some(self.crtime)
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub fn mode(&self) -> Option<Mode> {
        self.valid.contains(ValidFlags::MODE).then_some(self.mode)
    }

    pub fn uid(&self) -> Option<Uid> {
        self.valid.contains(ValidFlags::UID).then_some(self.uid)
    }

    pub fn gid(&self) -> Option<Gid> {
        self.valid.contains(ValidFlags::GID).then_some(self.gid)
    }

    /// Whether to clear the setuid bit, and the setgid bit if the file is
    /// group-executable.
    ///
    /// Only sent when [`FsCaps::HANDLE_KILLPRIV_V2`](crate::types::FsCaps::HANDLE_KILLPRIV_V2)
    /// is enabled, and only by Linux: FreeBSD never sets it, and macOS predates
    /// it.
    pub fn remove_suid_sgid(&self) -> bool {
        self.valid.contains(ValidFlags::KILL_SUIDGID)
    }

    pub fn flags(&self) -> Option<FileFlag> {
        #[cfg(target_os = "macos")]
        {
            self.valid.contains(ValidFlags::FLAGS).then_some(self.flags)
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}

impl SetAttr {
    pub(super) fn decode(body: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if body.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(body.as_ptr().add(HDR_LEN) as *const Raw) };
        let valid = ValidFlags::from_bits_truncate(raw.valid);

        let atime = if valid.contains(ValidFlags::ATIME_NOW) || !valid.contains(ValidFlags::ATIME) {
            FileTime::Now
        } else {
            FileTime::Specific(make_time(raw.atime, raw.atimensec)?)
        };

        let mtime = if valid.contains(ValidFlags::MTIME_NOW) || !valid.contains(ValidFlags::MTIME) {
            FileTime::Now
        } else {
            FileTime::Specific(make_time(raw.mtime, raw.mtimensec)?)
        };

        // macFUSE maps its Darwin change time onto the portable one, so a
        // filesystem doesn't have to know which flag its kernel uses.
        #[cfg(target_os = "macos")]
        let ctime = if valid.contains(ValidFlags::CHGTIME) {
            make_time(raw.chgtime, raw.chgtimensec)?
        } else {
            SystemTime::UNIX_EPOCH
        };

        #[cfg(not(target_os = "macos"))]
        let ctime = if valid.contains(ValidFlags::CTIME) {
            make_time(raw.ctime, raw.ctimensec)?
        } else {
            SystemTime::UNIX_EPOCH
        };

        #[cfg(target_os = "macos")]
        let bkuptime;
        #[cfg(target_os = "macos")]
        let crtime;

        #[cfg(target_os = "macos")]
        {
            bkuptime = if valid.contains(ValidFlags::BKUPTIME) {
                make_time(raw.bkuptime, raw.bkuptimensec)?
            } else {
                SystemTime::UNIX_EPOCH
            };

            crtime = if valid.contains(ValidFlags::CRTIME) {
                make_time(raw.crtime, raw.crtimensec)?
            } else {
                SystemTime::UNIX_EPOCH
            };
        }

        Ok(Self {
            ino,
            valid,
            fh: FileHandle(raw.fh),
            size: raw.size,
            atime,
            mtime,
            ctime,
            // TODO(e2e): assumes host-native mode (and macOS file flag) values;
            // verify once end-to-end tests can be done.
            mode: Mode::from_bits_retain(raw.mode as nix::libc::mode_t),
            uid: Uid::from_raw(raw.uid),
            gid: Gid::from_raw(raw.gid),
            #[cfg(target_os = "macos")]
            bkuptime,
            #[cfg(target_os = "macos")]
            crtime,
            #[cfg(target_os = "macos")]
            flags: FileFlag::from_bits_retain(raw.flags),
        })
    }
}

fn make_time(secs: u64, nsecs: u32) -> Result<SystemTime> {
    crate::proto::time::join_raw(secs, nsecs)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    const KILL_SUIDGID: u32 = 1 << 11;
    const CTIME: u32 = 1 << 10;
    #[cfg(target_os = "macos")]
    const CHGTIME: u32 = 1 << 29;
    const MODE: u32 = 1 << 0;
    const SIZE: u32 = 1 << 3;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn decode(valid: u32) -> SetAttr {
        decode_with_times(valid, 0, 0)
    }

    fn decode_with_times(valid: u32, ctime: u64, chgtime: u64) -> SetAttr {
        let mut buf = BufPool::new().checkout_with_capacity(256);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&valid.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes()); // padding
        buf.extend_from_slice(&0u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&4096u64.to_ne_bytes()); // size
        buf.extend_from_slice(&0u64.to_ne_bytes()); // lock_owner
        buf.extend_from_slice(&0u64.to_ne_bytes()); // atime
        buf.extend_from_slice(&0u64.to_ne_bytes()); // mtime
        buf.extend_from_slice(&ctime.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes()); // atimensec
        buf.extend_from_slice(&0u32.to_ne_bytes()); // mtimensec
        buf.extend_from_slice(&0u32.to_ne_bytes()); // ctimensec
        buf.extend_from_slice(&0o640u32.to_ne_bytes()); // mode
        buf.extend_from_slice(&0u32.to_ne_bytes()); // unused
        buf.extend_from_slice(&0u32.to_ne_bytes()); // uid
        buf.extend_from_slice(&0u32.to_ne_bytes()); // gid
        buf.extend_from_slice(&0u32.to_ne_bytes()); // unused

        #[cfg(target_os = "macos")]
        {
            buf.extend_from_slice(&0u64.to_ne_bytes()); // bkuptime
            buf.extend_from_slice(&chgtime.to_ne_bytes());
            buf.extend_from_slice(&0u64.to_ne_bytes()); // crtime
            buf.extend_from_slice(&[0u8; 4 * 4]); // the nsec fields and flags
        }
        let _ = chgtime;

        SetAttr::decode(buf, Ino::from_raw(1), cfg()).unwrap()
    }

    #[test]
    fn the_kill_flag_is_read() {
        assert!(decode(KILL_SUIDGID).remove_suid_sgid());
    }

    #[test]
    fn without_the_flag_nothing_is_cleared() {
        assert!(!decode(0).remove_suid_sgid());
        assert!(!decode(MODE | SIZE).remove_suid_sgid());
    }

    #[test]
    fn the_unused_bit_beside_it_is_ignored() {
        // libfuse's `FUSE_SET_ATTR_KILL_SGID` names this bit, but it belongs to
        // the Linux VFS's internal flags and never reaches the wire.
        assert!(!decode(1 << 12).remove_suid_sgid());
    }

    #[test]
    fn the_other_fields_still_decode_alongside_it() {
        let req = decode(KILL_SUIDGID | MODE | SIZE);

        assert!(req.remove_suid_sgid());
        assert_eq!(req.size(), Some(4096));
        assert_eq!(req.mode().map(|mode| mode.bits()), Some(0o640));
    }

    #[test]
    fn no_flag_means_no_change_time() {
        assert!(decode(0).ctime().is_none());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_portable_flag_carries_it() {
        let req = decode_with_times(CTIME, 1_700_000_000, 0);

        assert_eq!(
            req.ctime(),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
        );
    }

    // macOS predates the portable flag and sends its own, which decodes to the
    // same accessor.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_darwin_flag_carries_it() {
        let req = decode_with_times(CHGTIME, 0, 1_700_000_000);

        assert_eq!(
            req.ctime(),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
        );
    }

    #[test]
    fn a_change_time_before_the_epoch_decodes() {
        // 1969-12-31T23:59:59.5, which the kernel sends as -1 seconds.
        let secs = (-1i64).cast_unsigned();

        #[cfg(target_os = "macos")]
        let req = decode_with_times(CHGTIME, 0, secs);
        #[cfg(not(target_os = "macos"))]
        let req = decode_with_times(CTIME, secs, 0);

        let ctime = req.ctime().unwrap();

        assert!(ctime < SystemTime::UNIX_EPOCH);
        assert_eq!(
            SystemTime::UNIX_EPOCH.duration_since(ctime).unwrap(),
            Duration::from_secs(1)
        );
    }
}
