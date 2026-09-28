use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Flush {
    ino: Ino,
    fh: FileHandle,
    lock_owner: Option<LockOwner>,
}

#[repr(C)]
struct Raw {
    fh: u64,
    _unused0: u32,
    _unused1: u32,
    lock_owner: u64,
}

/// The body before 7.7, which carries no lock owner.
#[repr(C)]
struct RawCompat {
    fh: u64,
    _unused0: u32,
    _unused1: u32,
}

impl Flush {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }
}

impl Flush {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 7 {
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

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            lock_owner: LockOwner::try_from(raw.lock_owner).ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            ..Cfg::default()
        }
    }

    fn request(lock_owner: Option<u64>) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(64);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&9u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&0u32.to_ne_bytes()); // unused
        buf.extend_from_slice(&0u32.to_ne_bytes()); // padding

        if let Some(lock_owner) = lock_owner {
            buf.extend_from_slice(&lock_owner.to_ne_bytes());
        }

        buf
    }

    #[test]
    fn the_body_decodes() {
        let req = Flush::decode(request(Some(42)), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
    }

    // Before 7.7 the body carries no lock owner.
    #[test]
    fn an_old_kernels_body_decodes() {
        let req = Flush::decode(request(None), Ino::from_raw(1), cfg(6)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert!(req.lock_owner().is_none());
    }

    #[test]
    fn a_short_body_is_rejected() {
        assert_eq!(
            Flush::decode(request(None), Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }
}
