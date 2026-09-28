use super::{Cfg, HDR_LEN, Ino};
use crate::types::{ReplyInitFlags, XattrMode};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct SetXattr {
    ino: Ino,
    xattr_mode: XattrMode,
    remove_sgid: bool,
    buf: Buf,
    key_start: usize,
    key_end: usize,
    #[cfg(target_os = "macos")]
    offset: usize,
}

#[repr(C)]
struct Raw {
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
    assert!(std::mem::size_of::<Raw>() == 8);

    #[cfg(target_os = "macos")]
    assert!(std::mem::size_of::<Raw>() == 16);
};

#[repr(C)]
struct RawExt {
    size: u32,
    flags: u32,
    setxattr_flags: RawFlags,
    _unused: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
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
        OsStr::from_bytes(&self.buf[self.key_start..self.key_end])
    }

    pub fn value(&self) -> &[u8] {
        &self.buf[(self.key_end + 1)..]
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
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        let (size, flags, setxattr_flags, key_offset) =
            if cfg.flags.contains(ReplyInitFlags::SETXATTR_EXT) {
                const KEY_OFFSET: usize = HDR_LEN + std::mem::size_of::<RawExt>();

                if buf.len() < KEY_OFFSET {
                    return Err(Error::EPROTO);
                }

                let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const RawExt) };

                (raw.size as usize, raw.flags, raw.setxattr_flags, KEY_OFFSET)
            } else {
                const KEY_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

                if buf.len() < KEY_OFFSET {
                    return Err(Error::EPROTO);
                }

                let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

                (raw.size as usize, raw.flags, RawFlags::empty(), KEY_OFFSET)
            };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        // macOS always carries `position`, since it never negotiates the
        // extended layout.
        #[cfg(target_os = "macos")]
        let offset = {
            let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };
            raw.position as usize
        };

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

        let Some(key_len) = memchr::memchr(0, &buf[key_offset..]) else {
            return Err(Error::EPROTO);
        };
        let key_end = key_offset + key_len;

        // The value follows the key's NUL terminator.
        let Some(value_end) = (key_end + 1).checked_add(size) else {
            return Err(Error::EPROTO);
        };

        if buf.len() < value_end {
            return Err(Error::EPROTO);
        }
        buf.truncate(value_end);

        let remove_sgid = setxattr_flags.contains(RawFlags::ACL_KILL_SGID)
            && &buf[key_offset..key_end] == b"system.posix_acl_access";

        Ok(Self {
            ino,
            xattr_mode,
            remove_sgid,
            buf,
            key_start: key_offset,
            key_end,
            #[cfg(target_os = "macos")]
            offset,
        })
    }
}

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

        buf.extend_from_slice(&[0u8; HDR_LEN]);

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
        buf.extend_from_slice(&[0u8; HDR_LEN]);
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
        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&0u32.to_ne_bytes());

        assert_eq!(
            SetXattr::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }
}
