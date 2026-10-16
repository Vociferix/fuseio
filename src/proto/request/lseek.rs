use super::{Cfg, Ino, RawHeader};
use crate::types::{FileHandle, Whence};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Lseek {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    whence: Whence,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    fh: u64,
    offset: u64,
    whence: u32,
    _unused: u32,
}

impl Lseek {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn whence(&self) -> Whence {
        self.whence
    }
}

impl Lseek {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        // TODO(e2e): assumes host-native SEEK_* values; verify once end-to-end
        // tests can be done.
        #[cfg(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        ))]
        use nix::libc::{SEEK_DATA, SEEK_HOLE};
        // Hosts without SEEK_DATA/SEEK_HOLE (e.g. NetBSD, whose FUSE predates
        // LSEEK) use Linux's values.
        #[cfg(not(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        )))]
        const SEEK_DATA: i32 = 3;
        #[cfg(not(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        )))]
        const SEEK_HOLE: i32 = 4;

        let whence = match raw.whence.cast_signed() {
            SEEK_DATA => Whence::Data,
            SEEK_HOLE => Whence::Hole,
            _ => return Err(Error::EINVAL),
        };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            offset: raw.offset,
            whence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_lseek_in { fh, offset, whence, padding }`.
    fn body(fh: u64, offset: u64, whence: u32) -> Vec<u8> {
        let mut body = fh.to_ne_bytes().to_vec();
        body.extend_from_slice(&offset.to_ne_bytes());
        body.extend_from_slice(&whence.to_ne_bytes());
        body.extend_from_slice(&0u32.to_ne_bytes());
        body
    }

    fn decode(whence: u32) -> Result<Lseek> {
        Lseek::decode(
            request(Opcode::LSEEK, &body(9, 4096, whence)),
            root(),
            latest(),
        )
    }

    #[test]
    fn the_handle_and_offset_come_from_the_body() {
        let req = decode(nix::libc::SEEK_DATA as u32).expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.file_handle().0, 9);
        assert_eq!(req.offset(), 4096);
    }

    // `fuse_file_llseek` answers `SEEK_SET`, `SEEK_CUR` and `SEEK_END` itself
    // and only ever forwards the other two, so one of them arriving is a peer
    // saying something the protocol does not allow.
    #[test]
    fn the_positions_the_kernel_handles_itself_are_refused() {
        for whence in [
            nix::libc::SEEK_SET,
            nix::libc::SEEK_CUR,
            nix::libc::SEEK_END,
        ] {
            assert_eq!(
                decode(whence as u32).map(|_| ()).unwrap_err(),
                Error::EINVAL,
                "whence {whence}"
            );
        }
    }

    // Only `SEEK_DATA` and `SEEK_HOLE` reach a filesystem: the kernel answers
    // the others itself. Their values differ between Linux and the BSDs, so
    // the host's own are what the peer sends.
    #[test]
    fn the_data_and_hole_positions_are_recognised() {
        assert_eq!(
            decode(nix::libc::SEEK_DATA as u32).unwrap().whence(),
            Whence::Data
        );
        assert_eq!(
            decode(nix::libc::SEEK_HOLE as u32).unwrap().whence(),
            Whence::Hole
        );
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Lseek::decode(request(Opcode::LSEEK, &[0u8; 8]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
