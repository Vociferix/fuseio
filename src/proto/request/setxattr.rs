use super::{Cfg, HDR_LEN, Ino};
use crate::types::{ReplyInitFlags, XattrMode};
use crate::{Buf, Error, Result};

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
}

#[repr(C)]
struct Raw {
    size: u32,
    flags: u32,
}

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

    pub fn mode(&self) -> XattrMode {
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

        if buf.len() < key_offset + key_len + size {
            return Err(Error::EPROTO);
        }
        buf.truncate(key_offset + key_len + size);

        let remove_sgid = setxattr_flags.contains(RawFlags::ACL_KILL_SGID)
            && &buf[key_offset..key_end] == b"system.posix_acl_access";

        Ok(Self {
            ino,
            xattr_mode,
            remove_sgid,
            buf,
            key_start: key_offset,
            key_end,
        })
    }
}
