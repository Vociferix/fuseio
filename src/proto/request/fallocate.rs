use super::{Cfg, Ino, RawHeader};
use crate::types::{FallocateFlags, FileHandle};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Fallocate {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: u64,
    flags: FallocateFlags,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    fh: u64,
    offset: u64,
    len: u64,
    mode: u32,
    _unused: u32,
}

impl Fallocate {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn flags(&self) -> FallocateFlags {
        self.flags
    }
}

impl Fallocate {
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
            offset: raw.offset,
            len: raw.len,
            // TODO(e2e): assumes host-native FALLOC_FL_* values (Linux's where the
            // host has none); verify once end-to-end tests can be done.
            flags: FallocateFlags::from_bits_retain(raw.mode.cast_signed()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_fallocate_in { fh, offset, length, mode, padding }`.
    fn body(fh: u64, offset: u64, len: u64, mode: u32) -> Vec<u8> {
        let mut body = fh.to_ne_bytes().to_vec();
        body.extend_from_slice(&offset.to_ne_bytes());
        body.extend_from_slice(&len.to_ne_bytes());
        body.extend_from_slice(&mode.to_ne_bytes());
        body.extend_from_slice(&0u32.to_ne_bytes());
        body
    }

    #[test]
    fn the_range_and_mode_come_from_the_body() {
        let req = Fallocate::decode(
            request(Opcode::FALLOCATE, &body(4, 1024, 8192, 0)),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.file_handle().0, 4);
        assert_eq!(req.offset(), 1024);
        assert_eq!(req.len(), 8192);
        assert!(
            req.flags().is_empty(),
            "no mode bits means a plain allocate"
        );
    }

    // A zero length allocates nothing, which is a request the filesystem still
    // has to be told about rather than one to reject here.
    #[test]
    fn a_zero_length_still_decodes() {
        let req = Fallocate::decode(
            request(Opcode::FALLOCATE, &body(1, 0, 0, 0)),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.len(), 0);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Fallocate::decode(request(Opcode::FALLOCATE, &[0u8; 16]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
