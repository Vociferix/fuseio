use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Release {
    ino: Ino,
    fh: FileHandle,
    flags: OFlag,
    flush: bool,
    flock_unlock: bool,
    lock_owner: Option<LockOwner>,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawFlags: u32 {
        const FLUSH = 1 << 0;
        const FLOCK_UNLOCK = 1 << 1;
    }
}

#[repr(C)]
struct Raw {
    fh: u64,
    flags: u32,
    release_flags: RawFlags,
    lock_owner: u64,
}

/// The body before 7.8, which carries neither release flags nor a lock owner.
#[repr(C)]
struct RawCompat {
    fh: u64,
    flags: u32,
    _unused: u32,
}

impl Release {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    /// The flags the file was opened with.
    pub fn open_flags(&self) -> OFlag {
        self.flags
    }

    /// Whether the filesystem should flush the file before closing it.
    pub fn should_flush(&self) -> bool {
        self.flush
    }

    /// Whether this also releases the file's `flock`.
    pub fn releases_flock(&self) -> bool {
        self.flock_unlock
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }
}

impl Release {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        // TODO(e2e): assumes host-native open flag values; verify once end-to-end
        // tests can be done.
        if cfg.minor_ver < 8 {
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
                flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
                flush: false,
                flock_unlock: false,
                lock_owner: None,
            });
        }

        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };
        let flush = raw.release_flags.contains(RawFlags::FLUSH);
        let flock_unlock = raw.release_flags.contains(RawFlags::FLOCK_UNLOCK);
        let lock_owner = LockOwner::try_from(raw.lock_owner).ok();

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            flush,
            flock_unlock,
            lock_owner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;

    const FLUSH: u32 = 1 << 0;
    const FLOCK_UNLOCK: u32 = 1 << 1;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            ..Cfg::default()
        }
    }

    fn request(release_flags: Option<u32>, lock_owner: u64) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(64);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&9u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&(nix::libc::O_RDONLY as u32).to_ne_bytes());

        match release_flags {
            Some(flags) => {
                buf.extend_from_slice(&flags.to_ne_bytes());
                buf.extend_from_slice(&lock_owner.to_ne_bytes());
            }
            None => buf.extend_from_slice(&0u32.to_ne_bytes()), // padding
        }

        buf
    }

    #[test]
    fn the_body_decodes() {
        let req = Release::decode(request(Some(FLUSH), 42), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert!(req.should_flush());
        assert!(!req.releases_flock());
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
    }

    #[test]
    fn an_flock_unlock_is_read() {
        let req =
            Release::decode(request(Some(FLOCK_UNLOCK), 42), Ino::from_raw(1), cfg(45)).unwrap();

        assert!(req.releases_flock());
        assert!(!req.should_flush());
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
    }

    // Before 7.8 the body stops after the open flags.
    #[test]
    fn an_old_kernels_body_decodes() {
        let req = Release::decode(request(None, 0), Ino::from_raw(1), cfg(7)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert!(!req.should_flush());
        assert!(!req.releases_flock());
        assert!(req.lock_owner().is_none());
    }

    #[test]
    fn a_short_body_is_rejected() {
        assert_eq!(
            Release::decode(request(None, 0), Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }
}
