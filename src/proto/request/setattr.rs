use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileFlag, FileHandle, FileTime, Gid, Mode, Uid};
use crate::{Error, Result, buf::Buf};

use std::time::{Duration, SystemTime};

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
        //const FORCE = 1 << 9;
        const CTIME = 1 << 10;
        const KILL_SUID = 1 << 11;
        const KILL_SGID = 1 << 12;
        //const FILE = 1 << 13;
        //const KILL_PRIV = 1 << 14;
        //const OPEN = 1 << 15;
        //const TIMES_SET = 1 << 16;
        //const TOUCH = 1 << 17;

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
    chgtime: SystemTime,
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

    pub fn ctime(&self) -> Option<SystemTime> {
        self.valid.contains(ValidFlags::CTIME).then_some(self.ctime)
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

    pub fn chgtime(&self) -> Option<SystemTime> {
        #[cfg(target_os = "macos")]
        {
            self.valid
                .contains(ValidFlags::CHGTIME)
                .then_some(self.chgtime)
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

    pub fn remove_suid(&self) -> bool {
        self.valid.contains(ValidFlags::KILL_SUID)
    }

    pub fn remove_sgid(&self) -> bool {
        self.valid.contains(ValidFlags::KILL_SGID)
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

        let ctime = if valid.contains(ValidFlags::CTIME) {
            make_time(raw.ctime, raw.ctimensec)?
        } else {
            SystemTime::UNIX_EPOCH
        };

        #[cfg(target_os = "macos")]
        let bkuptime;
        #[cfg(target_os = "macos")]
        let chgtime;
        #[cfg(target_os = "macos")]
        let crtime;

        #[cfg(target_os = "macos")]
        {
            bkuptime = if valid.contains(ValidFlags::BKUPTIME) {
                make_time(raw.bkuptime, raw.bkuptimensec)?
            } else {
                SystemTime::UNIX_EPOCH
            };

            chgtime = if valid.contains(ValidFlags::CHGTIME) {
                make_time(raw.chgtime, raw.chgtimensec)?
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
            chgtime,
            #[cfg(target_os = "macos")]
            crtime,
            #[cfg(target_os = "macos")]
            flags: FileFlag::from_bits_retain(raw.flags),
        })
    }
}

fn make_time(secs: u64, nsecs: u32) -> Result<SystemTime> {
    if nsecs >= 1_000_000_000 {
        return Err(Error::EINVAL);
    }

    Duration::from_secs(secs)
        .checked_add(Duration::from_nanos(nsecs.into()))
        .and_then(|ts| SystemTime::UNIX_EPOCH.checked_add(ts))
        .ok_or(Error::EINVAL)
}
