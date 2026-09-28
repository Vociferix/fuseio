use super::{Cfg, HDR_LEN, Ino};
use crate::types::RenameMode;
use crate::{Error, Result, buf::Buf};

#[cfg(target_os = "macos")]
use crate::types::ReplyInitFlags;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

/// A rename, decoded from `FUSE_RENAME`, `FUSE_RENAME2`, or macOS
/// `FUSE_EXCHANGE`.
#[derive(Debug)]
pub struct Rename {
    old_parent: Ino,
    new_parent: Ino,
    buf: Buf,
    old_name_start: usize,
    old_name_end: usize,
    mode: RenameMode,
}

impl Rename {
    pub fn old_parent(&self) -> Ino {
        self.old_parent
    }

    pub fn new_parent(&self) -> Ino {
        self.new_parent
    }

    pub fn old_name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[self.old_name_start..self.old_name_end])
    }

    pub fn new_name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[(self.old_name_end + 1)..])
    }

    pub fn rename_mode(&self) -> RenameMode {
        self.mode
    }
}

/// `fuse_rename2_in`, also sent by macOS for `FUSE_RENAME` when the rename flags
/// are negotiated.
#[repr(C)]
struct RawExt {
    newdir: u64,
    flags: u32,
    _unused: u32,
}

#[repr(C)]
struct RawExchange {
    old_dir: u64,
    new_dir: u64,
    _options: u64,
}

const COMPAT_NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<u64>();
const EXT_NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<RawExt>();

impl Rename {
    /// Decodes `FUSE_RENAME`.
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        // The macOS kext's layout (8-byte compat vs `newdir, flags, padding`)
        // is keyed off the kernel's offer by macFUSE's 2.9 library and off the
        // negotiated reply by its 3.18 library. These flags are always accepted
        // when offered (see `ReplyInitFlags::negotiate`), so both agree.
        #[cfg(target_os = "macos")]
        if cfg.flags.intersects(
            ReplyInitFlags::DARWIN_RENAME_SWAP.union(ReplyInitFlags::DARWIN_RENAME_EXCL),
        ) {
            let (newdir, flags) = read_ext(&buf)?;

            // TODO(e2e): assumes host-native renamex_np(2) flags; verify once
            // end-to-end tests can be done.
            let mode = match flags {
                0 => RenameMode::Replace,
                nix::libc::RENAME_SWAP => RenameMode::Exchange,
                nix::libc::RENAME_EXCL => RenameMode::NoReplace,
                _ => return Err(Error::EINVAL),
            };

            return Self::from_parts(buf, ino, newdir, EXT_NAME_OFFSET, mode);
        }

        if buf.len() < COMPAT_NAME_OFFSET {
            return Err(Error::EPROTO);
        }

        let newdir = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const u64) };

        Self::from_parts(buf, ino, newdir, COMPAT_NAME_OFFSET, RenameMode::Replace)
    }

    /// Decodes `FUSE_RENAME2`.
    pub(super) fn decode_rename2(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let (newdir, flags) = read_ext(&buf)?;

        // TODO(e2e): assumes the kernel sends host-native renameat2(2) flags;
        // verify once end-to-end tests can be done.
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const RENAME_NOREPLACE: u32 = nix::libc::RENAME_NOREPLACE as u32;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const RENAME_EXCHANGE: u32 = nix::libc::RENAME_EXCHANGE as u32;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const RENAME_WHITEOUT: u32 = nix::libc::RENAME_WHITEOUT as u32;
        // Only Linux sends RENAME2; other hosts have no native values, so Linux's
        // are used.
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const RENAME_NOREPLACE: u32 = 1 << 0;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const RENAME_EXCHANGE: u32 = 1 << 1;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const RENAME_WHITEOUT: u32 = 1 << 2;
        const RENAME_WHITEOUT_NOREPLACE: u32 = RENAME_WHITEOUT | RENAME_NOREPLACE;

        let mode = match flags {
            0 => RenameMode::Replace,
            RENAME_NOREPLACE => RenameMode::NoReplace,
            RENAME_EXCHANGE => RenameMode::Exchange,
            RENAME_WHITEOUT => RenameMode::Whiteout,
            RENAME_WHITEOUT_NOREPLACE => RenameMode::WhiteoutNoReplace,
            _ => return Err(Error::EINVAL),
        };

        Self::from_parts(buf, ino, newdir, EXT_NAME_OFFSET, mode)
    }

    /// Decodes macOS `FUSE_EXCHANGE` (`exchangedata(2)`).
    ///
    /// Decodable everywhere: no kernel but macOS sends it, but its body has one
    /// layout, so a connection that speaks for a macFUSE peer can be served on
    /// any host.
    pub(super) fn decode_exchange(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        const NAMES_OFFSET: usize = HDR_LEN + std::mem::size_of::<RawExchange>();

        if buf.len() < NAMES_OFFSET {
            return Err(Error::EPROTO);
        }

        // TODO(e2e): `options` (exchangedata(2) FSOPT_* flags) is ignored on the
        // assumption that the kernel already applied them while resolving both
        // paths; verify once end-to-end tests can be done.
        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const RawExchange) };

        Self::from_parts(
            buf,
            Ino::from_raw(raw.old_dir),
            raw.new_dir,
            NAMES_OFFSET,
            RenameMode::ExchangeData,
        )
    }

    fn from_parts(
        mut buf: Buf,
        ino: Option<Ino>,
        newdir: u64,
        old_name_start: usize,
        mode: RenameMode,
    ) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(new_parent) = Ino::from_raw(newdir) else {
            return Err(Error::EINVAL);
        };

        let Some(old_name_len) = memchr::memchr(0, &buf[old_name_start..]) else {
            return Err(Error::EPROTO);
        };

        let old_name_end = old_name_start + old_name_len;

        if let Some(new_name_len) = memchr::memchr(0, &buf[(old_name_end + 1)..]) {
            buf.truncate(old_name_end + 1 + new_name_len);
        }

        Ok(Self {
            old_parent: ino,
            new_parent,
            buf,
            old_name_start,
            old_name_end,
            mode,
        })
    }
}

fn read_ext(buf: &Buf) -> Result<(u64, u32)> {
    if buf.len() < EXT_NAME_OFFSET {
        return Err(Error::EPROTO);
    }

    let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const RawExt) };

    Ok((raw.newdir, raw.flags))
}
