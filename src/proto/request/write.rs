use super::{Cfg, Ino, RawHeader};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

#[derive(Debug)]
pub struct Write {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    buf: BufView,
    lock_owner: Option<LockOwner>,
    flags: OFlag,
    cache_writeback: bool,
    remove_suid_sgid: bool,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::AnyBitPattern)]
    struct RawFlags: u32 {
        const CACHE = 1 << 0;
        const HAVE_LOCKOWNER = 1 << 1;
        const KILL_SUIDGID = 1 << 2;
    }
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    fh: u64,
    offset: u64,
    size: u32,
    write_flags: RawFlags,
    lock_owner: u64,
    flags: u32,
    _unused: u32,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawCompat {
    hdr: RawHeader,
    fh: u64,
    offset: u64,
    size: u32,
    write_flags: RawFlags,
}

impl Write {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }

    pub fn open_flags(&self) -> OFlag {
        self.flags
    }

    pub fn cache_writeback(&self) -> bool {
        self.cache_writeback
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.remove_suid_sgid
    }
}

impl Write {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 9 {
            let Some((raw, data)) = buf.as_slice().split_first::<RawCompat>() else {
                return Err(Error::EPROTO);
            };

            let size = raw.size as usize;
            let fh = raw.fh;
            let offset = raw.offset;
            let write_flags = raw.write_flags;

            if data.len() < size {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let Some(buf) = buf.try_into_view(
                std::mem::size_of::<RawCompat>()..(std::mem::size_of::<RawCompat>() + size),
            ) else {
                return Err(Error::EPROTO);
            };

            Ok(Self {
                ino,
                fh: FileHandle(fh),
                offset,
                buf,
                lock_owner: None,
                flags: OFlag::empty(),
                cache_writeback: write_flags.contains(RawFlags::CACHE),
                remove_suid_sgid: write_flags.contains(RawFlags::KILL_SUIDGID),
            })
        } else {
            let Some((raw, data)) = buf.as_slice().split_first::<Raw>() else {
                return Err(Error::EPROTO);
            };

            let size = raw.size as usize;
            let fh = raw.fh;
            let offset = raw.offset;
            let write_flags = raw.write_flags;
            let lock_owner = raw.lock_owner;
            let flags = raw.flags;

            if data.len() < size {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let Some(buf) =
                buf.try_into_view(std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + size))
            else {
                return Err(Error::EPROTO);
            };

            let lock_owner = if write_flags.contains(RawFlags::HAVE_LOCKOWNER) {
                LockOwner::try_from(lock_owner).ok()
            } else {
                None
            };

            Ok(Self {
                ino,
                fh: FileHandle(fh),
                offset,
                buf,
                lock_owner,
                // TODO(e2e): assumes host-native open flag values; verify once end-to-end
                // tests can be done.
                flags: OFlag::from_bits_retain(flags.cast_signed()),
                cache_writeback: write_flags.contains(RawFlags::CACHE),
                remove_suid_sgid: write_flags.contains(RawFlags::KILL_SUIDGID),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            ..Cfg::default()
        }
    }

    fn request(size: u32, data: &[u8], compat: bool) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(128);

        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);
        buf.extend_from_slice(&9u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&4096u64.to_ne_bytes()); // offset
        buf.extend_from_slice(&size.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes()); // write_flags

        if !compat {
            buf.extend_from_slice(&0u64.to_ne_bytes()); // lock_owner
            buf.extend_from_slice(&0u32.to_ne_bytes()); // flags
            buf.extend_from_slice(&0u32.to_ne_bytes()); // padding
        }

        buf.extend_from_slice(data);

        buf
    }

    #[test]
    fn the_payload_decodes() {
        let req = Write::decode(request(5, b"hello", false), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert_eq!(req.offset(), 4096);
        assert_eq!(req.data(), b"hello");
    }

    #[test]
    fn an_empty_payload_decodes() {
        let req = Write::decode(request(0, b"", false), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.data(), b"");
    }

    #[test]
    fn a_payload_shorter_than_its_size_is_rejected() {
        let buf = request(6, b"hello", false);

        assert_eq!(
            Write::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn an_absurd_size_is_rejected() {
        let buf = request(u32::MAX, b"hello", false);

        assert_eq!(
            Write::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }

    // Before 7.9 the header stops after the write flags.
    #[test]
    fn an_old_kernels_payload_decodes() {
        let req = Write::decode(request(5, b"hello", true), Ino::from_raw(1), cfg(8)).unwrap();

        assert_eq!(req.data(), b"hello");
        assert!(req.lock_owner().is_none());
    }

    #[test]
    fn a_payload_shorter_than_its_size_is_rejected_on_an_old_kernel() {
        let buf = request(6, b"hello", true);

        assert_eq!(
            Write::decode(buf, Ino::from_raw(1), cfg(8)).unwrap_err(),
            Error::EPROTO
        );
    }
}
