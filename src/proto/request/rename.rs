use super::{Cfg, Ino, RawHeader};
use crate::types::RenameMode;
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

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
    buf: BufView,
    old_name_len: usize,
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
        OsStr::from_bytes(&self.buf[..self.old_name_len])
    }

    pub fn new_name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[(self.old_name_len + 1)..])
    }

    pub fn rename_mode(&self) -> RenameMode {
        self.mode
    }
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawCompat {
    hdr: RawHeader,
    newdir: u64,
}

/// `fuse_rename2_in`, also sent by macOS for `FUSE_RENAME` when the rename flags
/// are negotiated.
#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawExt {
    hdr: RawHeader,
    newdir: u64,
    flags: u32,
    _unused: u32,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawExchange {
    hdr: RawHeader,
    old_dir: u64,
    new_dir: u64,
    _options: u64,
}

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
            let (newdir, flags, names, old_name_len) = read_ext(buf)?;

            // TODO(e2e): assumes host-native renamex_np(2) flags; verify once
            // end-to-end tests can be done.
            let mode = match flags {
                0 => RenameMode::Replace,
                nix::libc::RENAME_SWAP => RenameMode::Exchange,
                nix::libc::RENAME_EXCL => RenameMode::NoReplace,
                _ => return Err(Error::EINVAL),
            };

            return Self::from_parts(ino, newdir, mode, names, old_name_len);
        }

        let Some((raw, names)) = buf.as_slice().split_first::<RawCompat>() else {
            return Err(Error::EPROTO);
        };

        let newdir = raw.newdir;

        let Some(old_name_len) = memchr::memchr(0, names) else {
            return Err(Error::EPROTO);
        };

        let old_name = &names[(old_name_len + 1)..];
        let Some(buf) = (if let Some(new_name_len) = memchr::memchr(0, old_name) {
            buf.try_into_view(
                std::mem::size_of::<RawCompat>()
                    ..(std::mem::size_of::<RawCompat>() + old_name_len + 1 + new_name_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawCompat>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Self::from_parts(ino, newdir, RenameMode::Replace, buf, old_name_len)
    }

    /// Decodes `FUSE_RENAME2`.
    // `RENAME_*` are `c_uint` on Linux and defined here elsewhere.
    #[allow(clippy::unnecessary_cast)]
    pub(super) fn decode_rename2(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let (newdir, flags, names, old_name_len) = read_ext(buf)?;

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

        Self::from_parts(ino, newdir, mode, names, old_name_len)
    }

    /// Decodes macOS `FUSE_EXCHANGE` (`exchangedata(2)`).
    ///
    /// Decodable everywhere: no kernel but macOS sends it, but its body has one
    /// layout, so a connection that speaks for a macFUSE peer can be served on
    /// any host.
    pub(super) fn decode_exchange(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        // TODO(e2e): `options` (exchangedata(2) FSOPT_* flags) is ignored on the
        // assumption that the kernel already applied them while resolving both
        // paths; verify once end-to-end tests can be done.
        let Some((raw, names)) = buf.as_slice().split_first::<RawExchange>() else {
            return Err(Error::EPROTO);
        };

        let old_dir = raw.old_dir;
        let new_dir = raw.new_dir;

        let Some(old_name_len) = memchr::memchr(0, names) else {
            return Err(Error::EPROTO);
        };

        let old_name = &names[(old_name_len + 1)..];
        let Some(buf) = (if let Some(new_name_len) = memchr::memchr(0, old_name) {
            buf.try_into_view(
                std::mem::size_of::<RawExchange>()
                    ..(std::mem::size_of::<RawExchange>() + old_name_len + 1 + new_name_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawExchange>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Self::from_parts(
            Ino::from_raw(old_dir),
            new_dir,
            RenameMode::ExchangeData,
            buf,
            old_name_len,
        )
    }

    fn from_parts(
        ino: Option<Ino>,
        newdir: u64,
        mode: RenameMode,
        names: BufView,
        old_name_len: usize,
    ) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(new_parent) = Ino::from_raw(newdir) else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            old_parent: ino,
            new_parent,
            buf: names,
            old_name_len,
            mode,
        })
    }
}

fn read_ext(buf: Buf) -> Result<(u64, u32, BufView, usize)> {
    let Some((raw, names)) = buf.as_slice().split_first::<RawExt>() else {
        return Err(Error::EPROTO);
    };

    let newdir = raw.newdir;
    let flags = raw.flags;

    let Some(old_name_len) = memchr::memchr(0, names) else {
        return Err(Error::EPROTO);
    };

    let old_name = &names[(old_name_len + 1)..];
    let Some(buf) = (if let Some(new_name_len) = memchr::memchr(0, old_name) {
        buf.try_into_view(
            std::mem::size_of::<RawExt>()
                ..(std::mem::size_of::<RawExt>() + old_name_len + 1 + new_name_len),
        )
    } else {
        buf.try_into_view(std::mem::size_of::<RawExt>()..)
    }) else {
        return Err(Error::EPROTO);
    };

    Ok((newdir, flags, buf, old_name_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_rename_in`: the destination directory, then both names.
    fn compat_body(newdir: u64, names: &[u8]) -> Vec<u8> {
        let mut body = newdir.to_ne_bytes().to_vec();
        body.extend_from_slice(names);
        body
    }

    /// `fuse_rename2_in`: the destination directory, the flags, then both names.
    fn ext_body(newdir: u64, flags: u32, names: &[u8]) -> Vec<u8> {
        let mut body = newdir.to_ne_bytes().to_vec();
        body.extend_from_slice(&flags.to_ne_bytes());
        body.extend_from_slice(&0u32.to_ne_bytes()); // padding
        body.extend_from_slice(names);
        body
    }

    /// macOS `FUSE_EXCHANGE`: both directories and the `exchangedata` options.
    fn exchange_body(old_dir: u64, new_dir: u64, names: &[u8]) -> Vec<u8> {
        let mut body = old_dir.to_ne_bytes().to_vec();
        body.extend_from_slice(&new_dir.to_ne_bytes());
        body.extend_from_slice(&0u64.to_ne_bytes()); // options
        body.extend_from_slice(names);
        body
    }

    fn rename2(flags: u32) -> Result<Rename> {
        Rename::decode_rename2(
            request(Opcode::RENAME2, &ext_body(2, flags, b"old\0new\0")),
            root(),
            latest(),
        )
    }

    // `FUSE_RENAME` carries only the destination directory, so the move is
    // always a plain replace.
    #[test]
    fn a_plain_rename_names_both_directories_and_both_names() {
        let req = Rename::decode(
            request(Opcode::RENAME, &compat_body(2, b"before\0after\0")),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.old_parent(), root().unwrap());
        assert_eq!(req.new_parent().as_raw(), 2);
        assert_eq!(req.old_name(), "before");
        assert_eq!(req.new_name(), "after");
        assert_eq!(req.rename_mode(), RenameMode::Replace);
    }

    // `renameat2`'s flags are what distinguish the modes, and the kernel sends
    // them verbatim.
    #[test]
    fn the_flags_choose_the_mode() {
        assert_eq!(rename2(0).unwrap().rename_mode(), RenameMode::Replace);
        assert_eq!(rename2(1).unwrap().rename_mode(), RenameMode::NoReplace);
        assert_eq!(rename2(2).unwrap().rename_mode(), RenameMode::Exchange);
        assert_eq!(rename2(4).unwrap().rename_mode(), RenameMode::Whiteout);
        assert_eq!(
            rename2(4 | 1).unwrap().rename_mode(),
            RenameMode::WhiteoutNoReplace
        );
    }

    // A combination no `renameat2` accepts, and a bit nothing defines, are both
    // requests this crate cannot honour rather than ones to guess at.
    #[test]
    fn a_flag_combination_that_means_nothing_is_refused() {
        assert_eq!(rename2(1 | 2).unwrap_err(), Error::EINVAL);
        assert_eq!(rename2(1 << 20).unwrap_err(), Error::EINVAL);
    }

    #[test]
    fn a_rename2_names_both_directories_and_both_names() {
        let req = rename2(0).expect("decodes");

        assert_eq!(req.old_parent(), root().unwrap());
        assert_eq!(req.new_parent().as_raw(), 2);
        assert_eq!(req.old_name(), "old");
        assert_eq!(req.new_name(), "new");
    }

    // `FUSE_EXCHANGE` takes both directories from its own body rather than the
    // header, since neither is the request's inode.
    #[test]
    fn an_exchange_takes_both_directories_from_its_body() {
        let req = Rename::decode_exchange(
            request(Opcode::EXCHANGE, &exchange_body(3, 4, b"one\0two\0")),
            None,
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.old_parent().as_raw(), 3);
        assert_eq!(req.new_parent().as_raw(), 4);
        assert_eq!(req.old_name(), "one");
        assert_eq!(req.new_name(), "two");
        assert_eq!(req.rename_mode(), RenameMode::ExchangeData);
    }

    #[test]
    fn a_destination_directory_of_zero_is_refused() {
        let err = Rename::decode(
            request(Opcode::RENAME, &compat_body(0, b"a\0b\0")),
            root(),
            latest(),
        )
        .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }

    #[test]
    fn an_exchange_with_a_zero_directory_is_refused() {
        for (old, new) in [(0, 4), (3, 0)] {
            let err = Rename::decode_exchange(
                request(Opcode::EXCHANGE, &exchange_body(old, new, b"a\0b\0")),
                None,
                latest(),
            )
            .unwrap_err();

            assert_eq!(err, Error::EINVAL, "old={old} new={new}");
        }
    }

    // Without the terminator between them there is no telling where the first
    // name ends.
    #[test]
    fn names_with_no_separator_are_refused() {
        for buf in [
            Rename::decode(
                request(Opcode::RENAME, &compat_body(2, b"nonul")),
                root(),
                latest(),
            ),
            Rename::decode_rename2(
                request(Opcode::RENAME2, &ext_body(2, 0, b"nonul")),
                root(),
                latest(),
            ),
            Rename::decode_exchange(
                request(Opcode::EXCHANGE, &exchange_body(3, 4, b"nonul")),
                None,
                latest(),
            ),
        ] {
            assert_eq!(buf.map(|_| ()).unwrap_err(), Error::EPROTO);
        }
    }

    // The second name runs to the end of the request, so a kernel that left its
    // terminator off is still unambiguous.
    #[test]
    fn an_unterminated_second_name_is_taken_as_the_rest() {
        let req = Rename::decode(
            request(Opcode::RENAME, &compat_body(2, b"old\0new")),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.old_name(), "old");
        assert_eq!(req.new_name(), "new");
    }

    #[test]
    fn a_body_too_short_for_its_layout_is_refused() {
        assert_eq!(
            Rename::decode(request(Opcode::RENAME, &[0u8; 4]), root(), latest())
                .map(|_| ())
                .unwrap_err(),
            Error::EPROTO
        );
        assert_eq!(
            Rename::decode_rename2(request(Opcode::RENAME2, &[0u8; 12]), root(), latest())
                .map(|_| ())
                .unwrap_err(),
            Error::EPROTO
        );
        assert_eq!(
            Rename::decode_exchange(request(Opcode::EXCHANGE, &[0u8; 20]), None, latest())
                .map(|_| ())
                .unwrap_err(),
            Error::EPROTO
        );
    }
}
