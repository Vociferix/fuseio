use super::getattr::GetAttrFlags;
use super::{Cfg, Ino, RawHeader};
use crate::types::{FileHandle, StatXMask, StatXSync};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct StatX {
    ino: Ino,
    fh: Option<FileHandle>,
    sync: StatXSync,
    mask: StatXMask,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    getattr_flags: GetAttrFlags,
    _unused: u32,
    fh: u64,
    flags: u32,
    mask: StatXMask,
}

impl StatX {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.fh
    }

    pub fn sync_mode(&self) -> StatXSync {
        self.sync
    }

    pub fn field_mask(&self) -> StatXMask {
        self.mask
    }
}

impl StatX {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        // Only Linux sends STATX, so Linux's values are the host-native ones.
        // Literals because libc omits them on older musl targets.
        const AT_STATX_SYNC_TYPE: i32 = 0x6000;
        const AT_STATX_SYNC_AS_STAT: i32 = 0x0000;
        const AT_STATX_FORCE_SYNC: i32 = 0x2000;
        const AT_STATX_DONT_SYNC: i32 = 0x4000;

        let sync = match raw.flags.cast_signed() & AT_STATX_SYNC_TYPE {
            AT_STATX_SYNC_AS_STAT => StatXSync::AsStat,
            AT_STATX_DONT_SYNC => StatXSync::DontSync,
            AT_STATX_FORCE_SYNC => StatXSync::Force,
            _ => return Err(Error::EINVAL),
        };

        Ok(Self {
            ino,
            fh: raw
                .getattr_flags
                .contains(GetAttrFlags::FH)
                .then_some(FileHandle(raw.fh)),
            sync,
            mask: raw.mask,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `AT_STATX_*` from the kernel's `fcntl.h`. Only Linux sends `FUSE_STATX`,
    /// so these are the values every peer uses.
    const SYNC_AS_STAT: u32 = 0x0000;
    const FORCE_SYNC: u32 = 0x2000;
    const DONT_SYNC: u32 = 0x4000;
    const SYNC_TYPE: u32 = 0x6000;

    /// `fuse_statx_in { getattr_flags, reserved, fh, sx_flags, sx_mask }`. The
    /// reserved word gets a value of its own so a misread shows up.
    fn body(getattr_flags: u32, fh: u64, flags: u32, mask: u32) -> Vec<u8> {
        let mut body = getattr_flags.to_ne_bytes().to_vec();
        body.extend_from_slice(&0xdead_beefu32.to_ne_bytes()); // reserved
        body.extend_from_slice(&fh.to_ne_bytes());
        body.extend_from_slice(&flags.to_ne_bytes());
        body.extend_from_slice(&mask.to_ne_bytes());
        body
    }

    fn decode(getattr_flags: u32, fh: u64, flags: u32, mask: u32) -> Result<StatX> {
        StatX::decode(
            request(Opcode::STATX, &body(getattr_flags, fh, flags, mask)),
            root(),
            latest(),
        )
    }

    #[test]
    fn the_requested_fields_come_from_the_mask() {
        let wanted = StatXMask::MODE | StatXMask::SIZE | StatXMask::MTIME;
        let req = decode(0, 0, SYNC_AS_STAT, wanted.bits()).expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.field_mask(), wanted);
    }

    // As in `FUSE_GETATTR`, the handle is only a handle when the flag says so.
    #[test]
    fn the_handle_is_ignored_unless_the_flag_says_otherwise() {
        assert_eq!(decode(0, 42, SYNC_AS_STAT, 0).unwrap().file_handle(), None);
        assert_eq!(
            decode(1, 42, SYNC_AS_STAT, 0)
                .unwrap()
                .file_handle()
                .map(|fh| fh.0),
            Some(42)
        );
    }

    // `AT_STATX_SYNC_TYPE` is a two-bit field with three meanings, and the
    // caller's choice decides whether the filesystem may answer from cache.
    #[test]
    fn each_sync_mode_is_recognised() {
        assert_eq!(
            decode(0, 0, SYNC_AS_STAT, 0).unwrap().sync_mode(),
            StatXSync::AsStat
        );
        assert_eq!(
            decode(0, 0, FORCE_SYNC, 0).unwrap().sync_mode(),
            StatXSync::Force
        );
        assert_eq!(
            decode(0, 0, DONT_SYNC, 0).unwrap().sync_mode(),
            StatXSync::DontSync
        );
    }

    // Both bits together is the one combination the field leaves undefined, so
    // guessing at it would answer a question nobody asked.
    #[test]
    fn a_sync_mode_the_kernel_does_not_define_is_refused() {
        assert_eq!(
            decode(0, 0, SYNC_TYPE, 0).map(|_| ()).unwrap_err(),
            Error::EINVAL
        );
    }

    // `sx_flags` carries more than the sync type, and the rest is not part of
    // the choice.
    #[test]
    fn the_other_flag_bits_do_not_change_the_sync_mode() {
        let noise = !SYNC_TYPE;

        assert_eq!(
            decode(0, 0, FORCE_SYNC | noise & !SYNC_TYPE, 0)
                .unwrap()
                .sync_mode(),
            StatXSync::Force
        );
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = StatX::decode(request(Opcode::STATX, &[0u8; 16]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = StatX::decode(
            request(Opcode::STATX, &body(0, 0, SYNC_AS_STAT, 0)),
            None,
            latest(),
        )
        .map(|_| ())
        .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
