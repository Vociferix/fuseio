use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{DeviceNumber, FileFlag, Gid, Ino, InodeKind, Mode, SFlag, Uid};

use std::time::{Duration, SystemTime};

#[repr(C)]
#[derive(Debug, Default)]
pub struct Attrs {
    attr_valid: u64,
    attr_valid_nsec: u32,
    _unused: u32,
    attr: InodeAttrs,
}

#[repr(C)]
struct AttrsCompat {
    attr_valid: u64,
    attr_valid_nsec: u32,
    _unused: u32,
    attr: InodeAttrsCompat,
}

#[repr(C)]
#[derive(Debug)]
pub struct InodeAttrs {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    #[cfg(target_os = "macos")]
    crtime: u64,
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32,
    #[cfg(target_os = "macos")]
    crtimensec: u32,
    mode: u32,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: DeviceNumber,
    #[cfg(target_os = "macos")]
    chflags: FileFlag,
    blksize: u32,
    flags: AttrsFlags,
}

/// The `S_IFMT` bits, which hold the inode's kind rather than its permissions.
const FORMAT_MASK: u32 = SFlag::S_IFMT.bits() as u32;

const _: () = {
    #[cfg(not(target_os = "macos"))]
    assert!(std::mem::size_of::<InodeAttrs>() == 88);

    #[cfg(target_os = "macos")]
    assert!(std::mem::size_of::<InodeAttrs>() == 104);
};

#[repr(C)]
/// `fuse_attr` before 7.9, which ends before `blksize`.
///
/// Only its size is used: a reply writes the current layout and reports this
/// length, since the older one is a prefix of it.
pub(super) struct InodeAttrsCompat {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    #[cfg(target_os = "macos")]
    crtime: u64,
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32,
    #[cfg(target_os = "macos")]
    crtimensec: u32,
    mode: u32,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: DeviceNumber,
    #[cfg(target_os = "macos")]
    chflags: FileFlag,
}

const _: () = {
    // `FUSE_COMPAT_ATTR_OUT_SIZE` and `FUSE_COMPAT_ENTRY_OUT_SIZE`, which are 16
    // bytes larger on macOS.
    #[cfg(not(target_os = "macos"))]
    assert!(std::mem::size_of::<AttrsCompat>() == 96);
    #[cfg(target_os = "macos")]
    assert!(std::mem::size_of::<AttrsCompat>() == 112);

    // The older layout has to be a prefix of the current one.
    assert!(
        std::mem::size_of::<InodeAttrsCompat>() + 2 * std::mem::size_of::<u32>()
            == std::mem::size_of::<InodeAttrs>()
    );
};

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct AttrsFlags: u32 {
        const SUBMOUNT = 1 << 0;
        const DAX = 1 << 1;
    }
}

impl InodeAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ino(mut self, ino: Ino) -> Self {
        self.ino = ino.as_raw();
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn blocks(mut self, blocks: u64) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn atime(mut self, atime: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split_raw(atime);
        self.atime = secs;
        self.atimensec = nanos;
        self
    }

    pub fn mtime(mut self, mtime: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split_raw(mtime);
        self.mtime = secs;
        self.mtimensec = nanos;
        self
    }

    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split_raw(ctime);
        self.ctime = secs;
        self.ctimensec = nanos;
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        #[cfg(target_os = "macos")]
        {
            let (secs, nanos) = crate::proto::time::split_raw(crtime);
            self.crtime = secs;
            self.crtimensec = nanos;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = crtime;
        }

        self
    }

    // TODO: pre-1970 times are clamped to the epoch here; the wire fields are
    // signed seconds (libfuse passes `st_*time` through), with nsec in [0, 1e9).
    pub fn kind(mut self, kind: InodeKind) -> Self {
        // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
        // can be done.
        self.mode =
            (self.mode & !FORMAT_MASK) | (u32::from(SFlag::from(kind).bits()) & FORMAT_MASK);
        self
    }

    /// Sets the permission bits, leaving the inode's kind alone.
    pub fn mode(mut self, mode: Mode) -> Self {
        // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
        // can be done.
        self.mode = (self.mode & FORMAT_MASK) | (u32::from(mode.bits()) & !FORMAT_MASK);
        self
    }

    /// Sets the device a character or block device names.
    pub fn device_number(mut self, device: DeviceNumber) -> Self {
        self.rdev = device;
        self
    }

    pub fn hard_links(mut self, count: usize) -> Self {
        self.nlink = count.try_into().unwrap_or(u32::MAX);
        self
    }

    pub fn uid(mut self, uid: Uid) -> Self {
        self.uid = uid.as_raw();
        self
    }

    pub fn gid(mut self, gid: Gid) -> Self {
        self.gid = gid.as_raw();
        self
    }

    pub fn block_size(mut self, size: usize) -> Self {
        self.blksize = size.try_into().unwrap_or(u32::MAX);
        self
    }

    pub fn submount_root(mut self, is_submount_root: bool) -> Self {
        self.flags.set(AttrsFlags::SUBMOUNT, is_submount_root);
        self
    }

    pub fn flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags = flags;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }

    pub fn add_flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags.insert(flags);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }

    pub fn clear_flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags.remove(flags);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.attr_valid = ttl.as_secs();
        self.attr_valid_nsec = ttl.subsec_nanos();
        self
    }

    pub fn attrs(mut self, attrs: InodeAttrs) -> Self {
        self.attr = attrs;
        self
    }
}

impl EncodeResp for Attrs {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            attr: Attrs,
        }

        #[repr(C)]
        struct OutCompat {
            hdr: RawHeader,
            attr: AttrsCompat,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                self.hdr.len as usize
            }
        }

        let len = if !cfg!(target_os = "macos") && cfg.minor_ver < 9 {
            std::mem::size_of::<OutCompat>()
        } else {
            std::mem::size_of::<Out>()
        };

        Ok(Out {
            hdr: RawHeader {
                len: len as u32,
                err: 0,
                id,
            },
            attr: self,
        })
    }
}

impl Default for InodeAttrs {
    fn default() -> Self {
        #[allow(unused_mut)]
        let mut attrs: Self = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        #[cfg(target_os = "macos")]
        {
            attrs.crtime = u64::MAX;
            attrs.crtimensec = u32::MAX;
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    // `fuse_attr` follows the 16-byte header and `attr_valid`; macOS inserts
    // `crtime` and `crtimensec` ahead of the fields after them.
    const ATTR: usize = 16 + 16;
    const MODE: usize = ATTR + if cfg!(target_os = "macos") { 72 } else { 60 };
    const CTIME: usize = ATTR + 40;
    const CTIMENSEC: usize = ATTR + if cfg!(target_os = "macos") { 64 } else { 56 };

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn encode(attrs: InodeAttrs) -> Vec<u8> {
        let buf = Attrs::new()
            .attrs(attrs)
            .encode(7, cfg())
            .unwrap()
            .into_io_buf();

        buf.iter_slice().flatten().copied().collect()
    }

    fn mode_of(attrs: InodeAttrs) -> u32 {
        let bytes = encode(attrs);

        u32::from_ne_bytes(bytes[MODE..MODE + 4].try_into().unwrap())
    }

    #[test]
    fn permissions_reach_the_wire() {
        let mode = mode_of(InodeAttrs::new().mode(Mode::from_bits_truncate(0o640)));

        assert_eq!(mode & !FORMAT_MASK, 0o640);
    }

    #[test]
    fn the_kind_reaches_the_wire() {
        let mode = mode_of(InodeAttrs::new().kind(InodeKind::Dir));

        assert_eq!(mode & FORMAT_MASK, u32::from(SFlag::S_IFDIR.bits()));
    }

    #[test]
    fn the_kind_and_permissions_compose_in_either_order() {
        let perm = Mode::from_bits_truncate(0o755);
        let expected = u32::from(SFlag::S_IFDIR.bits()) | 0o755;

        assert_eq!(
            mode_of(InodeAttrs::new().kind(InodeKind::Dir).mode(perm)),
            expected
        );
        assert_eq!(
            mode_of(InodeAttrs::new().mode(perm).kind(InodeKind::Dir)),
            expected
        );
    }

    #[test]
    fn setting_the_kind_twice_replaces_it() {
        let attrs = InodeAttrs::new()
            .kind(InodeKind::Dir)
            .mode(Mode::from_bits_truncate(0o600))
            .kind(InodeKind::File);

        assert_eq!(mode_of(attrs), u32::from(SFlag::S_IFREG.bits()) | 0o600);
    }

    #[test]
    fn a_full_mode_cant_corrupt_the_kind() {
        let attrs = InodeAttrs::new()
            .kind(InodeKind::Fifo)
            .mode(Mode::from_bits_retain((FORMAT_MASK | 0o644) as _));

        assert_eq!(mode_of(attrs), u32::from(SFlag::S_IFIFO.bits()) | 0o644);
    }

    #[test]
    fn setuid_setgid_and_sticky_bits_survive() {
        let mode = Mode::S_ISUID | Mode::S_ISGID | Mode::S_ISVTX;
        let encoded = mode_of(InodeAttrs::new().kind(InodeKind::File).mode(mode));

        assert_eq!(encoded & !FORMAT_MASK, u32::from(mode.bits()));
    }

    #[test]
    fn a_time_before_the_epoch_survives_encoding() {
        let ctime = SystemTime::UNIX_EPOCH - Duration::from_millis(1500);
        let bytes = encode(InodeAttrs::new().ctime(ctime));

        let secs = i64::from_ne_bytes(bytes[CTIME..CTIME + 8].try_into().unwrap());
        let nanos = u32::from_ne_bytes(bytes[CTIMENSEC..CTIMENSEC + 4].try_into().unwrap());

        // -2 seconds plus 500 ms, since the remainder always moves forward.
        assert_eq!(secs, -2);
        assert_eq!(nanos, 500_000_000);
    }

    #[test]
    fn a_time_after_the_epoch_survives_encoding() {
        let ctime = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 250);
        let bytes = encode(InodeAttrs::new().ctime(ctime));

        let secs = i64::from_ne_bytes(bytes[CTIME..CTIME + 8].try_into().unwrap());
        let nanos = u32::from_ne_bytes(bytes[CTIMENSEC..CTIMENSEC + 4].try_into().unwrap());

        assert_eq!(secs, 1_700_000_000);
        assert_eq!(nanos, 250);
    }

    #[test]
    fn the_offsets_match_the_struct() {
        let attrs = InodeAttrs::new();
        let base = &attrs as *const InodeAttrs as usize;

        assert_eq!(&attrs.mode as *const u32 as usize - base, MODE - ATTR);
        assert_eq!(&attrs.ctime as *const u64 as usize - base, CTIME - ATTR);
        assert_eq!(
            &attrs.ctimensec as *const u32 as usize - base,
            CTIMENSEC - ATTR
        );
    }

    const RDEV: usize = ATTR + if cfg!(target_os = "macos") { 88 } else { 76 };

    #[test]
    fn a_device_number_reaches_the_wire() {
        let dev = crate::types::DeviceNumber::new(8, 3).unwrap();
        let bytes = encode(
            InodeAttrs::new()
                .kind(InodeKind::BlockDev)
                .device_number(dev),
        );

        let raw = u32::from_ne_bytes(bytes[RDEV..RDEV + 4].try_into().unwrap());

        assert_eq!(raw, dev.as_raw());
        assert_eq!(crate::types::DeviceNumber::from_raw(raw), dev);
    }

    #[test]
    fn the_device_offset_matches_the_struct() {
        let attrs = InodeAttrs::new();
        let base = &attrs as *const InodeAttrs as usize;

        assert_eq!(
            &attrs.rdev as *const DeviceNumber as usize - base,
            RDEV - ATTR
        );
    }
}
