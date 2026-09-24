use super::{Cfg, HDR_LEN, Ino};
use crate::types::{CopyFileRangePos, FileHandle};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct CopyFileRange {
    src: CopyFileRangePos,
    dst: CopyFileRangePos,
    len: u64,
}

#[repr(C)]
struct Raw {
    fh_in: u64,
    off_in: u64,
    ino_out: u64,
    fh_out: u64,
    off_out: u64,
    len: u64,
    flags: u64,
}

impl CopyFileRange {
    pub fn src(&self) -> &CopyFileRangePos {
        &self.src
    }

    pub fn dst(&self) -> &CopyFileRangePos {
        &self.dst
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

/// The largest length `FUSE_COPY_FILE_RANGE` can report back, since its reply
/// counts bytes in a `u32`.
///
/// libfuse clamps the request the same way.
pub(super) const MAX_COMPAT_LEN: u64 = 0xfffff000;

impl CopyFileRange {
    /// Decodes `FUSE_COPY_FILE_RANGE`, whose reply is a `fuse_write_out`.
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        let mut req = Self::decode_64(buf, ino, cfg)?;

        req.len = req.len.min(MAX_COMPAT_LEN);

        Ok(req)
    }

    /// Decodes `FUSE_COPY_FILE_RANGE_64`, whose reply can report the full range.
    pub(super) fn decode_64(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        let Some(ino_out) = Ino::from_raw(raw.ino_out) else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            src: CopyFileRangePos {
                ino,
                fh: FileHandle(raw.fh_in),
                offset: raw.off_in,
            },
            dst: CopyFileRangePos {
                ino: ino_out,
                fh: FileHandle(raw.fh_out),
                offset: raw.off_out,
            },
            len: raw.len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn request(len: u64) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(128);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&1u64.to_ne_bytes()); // fh_in
        buf.extend_from_slice(&2u64.to_ne_bytes()); // off_in
        buf.extend_from_slice(&3u64.to_ne_bytes()); // ino_out
        buf.extend_from_slice(&4u64.to_ne_bytes()); // fh_out
        buf.extend_from_slice(&5u64.to_ne_bytes()); // off_out
        buf.extend_from_slice(&len.to_ne_bytes());
        buf.extend_from_slice(&0u64.to_ne_bytes()); // flags

        buf
    }

    #[test]
    fn a_short_copy_is_left_alone() {
        let req = CopyFileRange::decode(request(4096), Ino::from_raw(9), cfg()).unwrap();

        assert_eq!(req.len(), 4096);
        assert_eq!(req.src().ino, Ino::from_raw(9).unwrap());
        assert_eq!(req.dst().ino, Ino::from_raw(3).unwrap());
    }

    #[test]
    fn a_copy_too_large_to_report_is_clamped() {
        let req = CopyFileRange::decode(request(u64::MAX), Ino::from_raw(9), cfg()).unwrap();

        assert_eq!(req.len(), MAX_COMPAT_LEN);
        assert!(u32::try_from(req.len()).is_ok());
    }

    #[test]
    fn the_64_bit_op_keeps_the_whole_length() {
        let req = CopyFileRange::decode_64(request(u64::MAX), Ino::from_raw(9), cfg()).unwrap();

        assert_eq!(req.len(), u64::MAX);
    }
}
