use super::{Cfg, Ino, RawHeader};
use crate::types::{ReplyInitFlags, XattrMode};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct SetXattr {
    ino: Ino,
    xattr_mode: XattrMode,
    remove_sgid: bool,
    buf: BufView,
    key_len: usize,
    #[cfg(target_os = "macos")]
    offset: usize,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    size: u32,
    flags: u32,
    #[cfg(target_os = "macos")]
    position: u32,
    #[cfg(target_os = "macos")]
    _unused: u32,
}

const _: () = {
    // macOS appends `position` and its padding.
    #[cfg(not(target_os = "macos"))]
    assert!(std::mem::size_of::<Raw>() == 8 + std::mem::size_of::<RawHeader>());

    #[cfg(target_os = "macos")]
    assert!(std::mem::size_of::<Raw>() == 16 + std::mem::size_of::<RawHeader>());
};

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawExt {
    hdr: RawHeader,
    size: u32,
    flags: u32,
    setxattr_flags: RawFlags,
    _unused: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::AnyBitPattern)]
    struct RawFlags: u32 {
        const ACL_KILL_SGID = 1 << 0;
    }
}

impl SetXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    /// Whether the attribute may be created, replaced, or either.
    pub fn xattr_mode(&self) -> XattrMode {
        self.xattr_mode
    }

    pub fn remove_sgid(&self) -> bool {
        self.remove_sgid
    }

    pub fn key(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[..self.key_len])
    }

    pub fn value(&self) -> &[u8] {
        &self.buf[(self.key_len + 1)..]
    }

    /// The offset within the attribute to write at.
    ///
    /// Only macOS sends one, and only for the resource fork; it is zero
    /// everywhere else.
    pub fn offset(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.offset
        }

        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }
}

impl SetXattr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        let (flags, setxattr_flags, offset, kv, key_len) =
            if cfg.flags.contains(ReplyInitFlags::SETXATTR_EXT) {
                let Some((raw, kv)) = buf.as_slice().split_first::<RawExt>() else {
                    return Err(Error::EPROTO);
                };

                let Some(key_len) = memchr::memchr(0, kv) else {
                    return Err(Error::EPROTO);
                };

                let size = raw.size as usize;
                let flags = raw.flags;
                let setxattr_flags = raw.setxattr_flags;

                let Some(buf) = buf.try_into_view(
                    std::mem::size_of::<RawExt>()
                        ..(std::mem::size_of::<RawExt>() + key_len + 1 + size),
                ) else {
                    return Err(Error::EPROTO);
                };

                (flags, setxattr_flags, 0, buf, key_len)
            } else {
                let Some((raw, kv)) = buf.as_slice().split_first::<Raw>() else {
                    return Err(Error::EPROTO);
                };

                let Some(key_len) = memchr::memchr(0, kv) else {
                    return Err(Error::EPROTO);
                };

                let size = raw.size as usize;
                let flags = raw.flags;

                // macOS always carries `position`, since it never negotiates the
                // extended layout.
                #[cfg(target_os = "macos")]
                let offset = raw.position as usize;

                #[cfg(not(target_os = "macos"))]
                let offset = 0usize;

                let Some(buf) = buf.try_into_view(
                    std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + key_len + 1 + size),
                ) else {
                    return Err(Error::EPROTO);
                };

                (flags, RawFlags::empty(), offset, buf, key_len)
            };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        #[cfg(not(target_os = "macos"))]
        let _ = offset;

        // TODO(e2e): assumes each kernel sends host-native setxattr(2) flags;
        // verify once end-to-end tests can be done.
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "netbsd"
        ))]
        use nix::libc::{XATTR_CREATE, XATTR_REPLACE};
        // FreeBSD's extattr API has no create/replace modes, so its libc has no
        // native values; Linux's are used.
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "netbsd"
        )))]
        const XATTR_CREATE: i32 = 0x1;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "netbsd"
        )))]
        const XATTR_REPLACE: i32 = 0x2;

        let xattr_mode = match flags.cast_signed() & (XATTR_CREATE | XATTR_REPLACE) {
            0 => XattrMode::CreateOrReplace,
            XATTR_CREATE => XattrMode::CreateOnly,
            XATTR_REPLACE => XattrMode::ReplaceOnly,
            _ => return Err(Error::EINVAL),
        };

        let remove_sgid = setxattr_flags.contains(RawFlags::ACL_KILL_SGID)
            && &kv[..key_len] == b"system.posix_acl_access";

        Ok(Self {
            ino,
            xattr_mode,
            remove_sgid,
            buf: kv,
            key_len,
            #[cfg(target_os = "macos")]
            offset,
        })
    }
}

/// The body length a request of this kind needs, which differs by
/// platform where the layout does.
#[cfg(test)]
pub(super) const RAW_LEN: usize =
    std::mem::size_of::<Raw>() - std::mem::size_of::<super::RawHeader>();

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;

    const KEY: &[u8] = b"user.test";

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn request_at(key: &[u8], value: &[u8], size: u32, position: u32) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(256);

        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);

        buf.extend_from_slice(&size.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes());
        #[cfg(target_os = "macos")]
        {
            buf.extend_from_slice(&position.to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes());
        }
        let _ = position;

        buf.extend_from_slice(key);
        buf.push(0);
        buf.extend_from_slice(value);

        buf
    }

    fn request(key: &[u8], value: &[u8], size: u32) -> Buf {
        request_at(key, value, size, 0)
    }

    fn decode(key: &[u8], value: &[u8]) -> Result<SetXattr> {
        let size = value.len() as u32;
        SetXattr::decode(request(key, value, size), Ino::from_raw(1), cfg())
    }

    #[test]
    fn decodes_the_key_and_the_whole_value() {
        let req = decode(KEY, b"hello").unwrap();

        assert_eq!(req.key().as_bytes(), KEY);
        assert_eq!(req.value(), b"hello");
    }

    #[test]
    fn decodes_an_empty_value() {
        let req = decode(KEY, b"").unwrap();

        assert_eq!(req.key().as_bytes(), KEY);
        assert_eq!(req.value(), b"");
    }

    #[test]
    fn decodes_the_offset() {
        let buf = request_at(KEY, b"hello", 5, 4096);
        let req = SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap();

        assert_eq!(req.key().as_bytes(), KEY);
        assert_eq!(req.value(), b"hello");
        assert_eq!(
            req.offset(),
            if cfg!(target_os = "macos") { 4096 } else { 0 }
        );
    }

    #[test]
    fn decodes_a_value_holding_nul_bytes() {
        let req = decode(KEY, b"a\0b").unwrap();

        assert_eq!(req.value(), b"a\0b");
    }

    #[test]
    fn rejects_a_value_shorter_than_its_size() {
        let buf = request(KEY, b"hello", 6);

        assert_eq!(
            SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn rejects_a_size_that_would_overflow() {
        let buf = request(KEY, b"hello", u32::MAX);

        assert_eq!(
            SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn rejects_an_unterminated_key() {
        let mut buf = BufPool::new().checkout_with_capacity(256);
        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes());
        #[cfg(target_os = "macos")]
        {
            buf.extend_from_slice(&0u32.to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes());
        }
        buf.extend_from_slice(KEY);

        assert_eq!(
            SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn rejects_a_truncated_body() {
        let mut buf = BufPool::new().checkout_with_capacity(256);
        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);
        buf.extend_from_slice(&0u32.to_ne_bytes());

        assert_eq!(
            SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }
}
