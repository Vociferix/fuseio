use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Read {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: usize,
    lock_owner: Option<LockOwner>,
    flags: OFlag,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Flags: u32 {
        const HAVE_LOCKOWNER = 1 << 1;
    }
}

#[repr(C)]
pub(super) struct Raw {
    fh: u64,
    offset: u64,
    size: u32,
    read_flags: Flags,
    lock_owner: u64,
    flags: u32,
    _unused: u32,
}

/// The body before 7.9, which carries neither a lock owner nor open flags.
#[repr(C)]
struct RawCompat {
    fh: u64,
    offset: u64,
    size: u32,
    read_flags: Flags,
}

impl Read {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }

    pub fn open_flags(&self) -> OFlag {
        self.flags
    }
}

impl Read {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 9 {
            if buf.len() < const { HDR_LEN + std::mem::size_of::<RawCompat>() } {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const RawCompat) };

            return Ok(Self {
                ino,
                fh: FileHandle(raw.fh),
                offset: raw.offset,
                len: raw.size as usize,
                lock_owner: None,
                flags: OFlag::empty(),
            });
        }

        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        let lock_owner = if raw.read_flags.contains(Flags::HAVE_LOCKOWNER) {
            LockOwner::try_from(raw.lock_owner).ok()
        } else {
            None
        };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            offset: raw.offset,
            len: raw.size as usize,
            lock_owner,
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    const HAVE_LOCKOWNER: u32 = 1 << 1;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            ..Cfg::default()
        }
    }

    fn request(read_flags: u32, lock_owner: u64, compat: bool) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(128);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&7u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&4096u64.to_ne_bytes()); // offset
        buf.extend_from_slice(&512u32.to_ne_bytes()); // size
        buf.extend_from_slice(&read_flags.to_ne_bytes());

        if !compat {
            buf.extend_from_slice(&lock_owner.to_ne_bytes());
            buf.extend_from_slice(&(nix::libc::O_RDONLY as u32).to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes()); // padding
        }

        buf
    }

    #[test]
    fn the_body_decodes() {
        let req = Read::decode(request(0, 0, false), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(7));
        assert_eq!(req.offset(), 4096);
        assert_eq!(req.len(), 512);
        assert!(req.lock_owner().is_none());
    }

    #[test]
    fn a_lock_owner_is_read_only_when_flagged() {
        let with = Read::decode(
            request(HAVE_LOCKOWNER, 99, false),
            Ino::from_raw(1),
            cfg(45),
        )
        .unwrap();
        let without = Read::decode(request(0, 99, false), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(with.lock_owner().map(u64::from), Some(99));
        assert!(without.lock_owner().is_none());
    }

    #[test]
    fn a_short_body_is_rejected() {
        let buf = request(0, 0, true);

        assert_eq!(
            Read::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }

    // Before 7.9 the body stops after `read_flags`.
    #[test]
    fn an_old_kernels_body_decodes() {
        let req = Read::decode(request(0, 0, true), Ino::from_raw(1), cfg(8)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(7));
        assert_eq!(req.offset(), 4096);
        assert_eq!(req.len(), 512);
        assert!(req.lock_owner().is_none());
        assert_eq!(req.open_flags(), OFlag::empty());
    }

    #[test]
    fn a_longer_body_is_accepted() {
        let mut buf = request(0, 0, false);
        buf.extend_from_slice(&[0u8; 16]);

        assert!(Read::decode(buf, Ino::from_raw(1), cfg(45)).is_ok());
    }
}
