use super::{Cfg, Ino, RawHeader};
use crate::types::FileHandle;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Fsync {
    ino: Ino,
    fh: FileHandle,
    datasync: bool,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    fh: u64,
    fsync_flags: RawFlags,
    _unused: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::AnyBitPattern)]
    struct RawFlags: u32 {
        const DATASYNC = 1 << 0;
    }
}

impl Fsync {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn datasync(&self) -> bool {
        self.datasync
    }
}

impl Fsync {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            datasync: raw.fsync_flags.contains(RawFlags::DATASYNC),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_fsync_in { fh, fsync_flags, padding }`.
    fn body(fh: u64, flags: u32) -> Vec<u8> {
        let mut body = fh.to_ne_bytes().to_vec();
        body.extend_from_slice(&flags.to_ne_bytes());
        body.extend_from_slice(&0xdead_beefu32.to_ne_bytes()); // padding
        body
    }

    /// `FUSE_FSYNC_FDATASYNC`: flush the data but not the metadata.
    const FDATASYNC: u32 = 1 << 0;

    #[test]
    fn a_full_sync_carries_its_handle() {
        let req =
            Fsync::decode(request(Opcode::FSYNC, &body(12, 0)), root(), latest()).expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.file_handle().0, 12);
        assert!(!req.datasync(), "no flag means metadata too");
    }

    #[test]
    fn the_flag_asks_for_the_data_alone() {
        let req = Fsync::decode(
            request(Opcode::FSYNC, &body(12, FDATASYNC)),
            root(),
            latest(),
        )
        .expect("decodes");

        assert!(req.datasync());
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Fsync::decode(request(Opcode::FSYNC, &[0u8; 8]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
