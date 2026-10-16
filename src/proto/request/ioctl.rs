use super::{Cfg, Ino, RawHeader};
use crate::types::{FileHandle, IoctlCmd, IoctlFlags};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

#[derive(Debug)]
pub struct Ioctl {
    ino: Ino,
    fh: FileHandle,
    flags: IoctlFlags,
    cmd: IoctlCmd,
    arg: u64,
    in_len: usize,
    out_len: usize,
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
pub struct Raw {
    hdr: RawHeader,
    fh: u64,
    flags: IoctlFlags,
    cmd: IoctlCmd,
    arg: u64,
    in_size: u32,
    out_size: u32,
}

impl Ioctl {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn flags(&self) -> IoctlFlags {
        self.flags
    }

    pub fn command(&self) -> IoctlCmd {
        self.cmd
    }

    pub fn arg(&self) -> u64 {
        self.arg
    }

    pub fn in_len(&self) -> usize {
        self.in_len
    }

    pub fn out_len(&self) -> usize {
        self.out_len
    }

    pub fn data(&self) -> &[u8] {
        &self.buf
    }
}

impl Ioctl {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let fh = FileHandle(raw.fh);
        let flags = raw.flags;
        let cmd = raw.cmd;
        let arg = raw.arg;
        let in_len = raw.in_size as usize;
        let out_len = raw.out_size as usize;

        let Some(buf) = buf.try_into_view(std::mem::size_of::<Raw>()..) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            ino,
            fh,
            flags,
            cmd,
            arg,
            in_len,
            out_len,
            buf,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_ioctl_in { fh, flags, cmd, arg, in_size, out_size }`, then the
    /// input the kernel gathered.
    fn body(cmd: u32, in_data: &[u8], out_size: u32) -> Vec<u8> {
        let mut body = 5u64.to_ne_bytes().to_vec(); // fh
        body.extend_from_slice(&0u32.to_ne_bytes()); // flags
        body.extend_from_slice(&cmd.to_ne_bytes());
        body.extend_from_slice(&0xdead_beefu64.to_ne_bytes()); // arg
        body.extend_from_slice(&(in_data.len() as u32).to_ne_bytes());
        body.extend_from_slice(&out_size.to_ne_bytes());
        body.extend_from_slice(in_data);
        body
    }

    fn decode(in_data: &[u8], out_size: u32) -> Result<Ioctl> {
        Ioctl::decode(
            request(Opcode::IOCTL, &body(0x1234, in_data, out_size)),
            root(),
            latest(),
        )
    }

    #[test]
    fn the_command_and_its_sizes_come_from_the_body() {
        let req = decode(b"input", 64).expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.file_handle().0, 5);
        assert_eq!(req.arg(), 0xdead_beef);
        assert_eq!(req.in_len(), 5);
        assert_eq!(req.out_len(), 64);
    }

    // The kernel sends exactly `in_size` bytes after the struct, so what the
    // filesystem reads has to be that and nothing else.
    #[test]
    fn the_input_is_exactly_what_was_announced() {
        let req = decode(b"input", 0).expect("decodes");

        assert_eq!(req.data(), b"input");
        assert_eq!(
            req.data().len(),
            req.in_len(),
            "the data and the announced size disagree"
        );
    }

    // An ioctl that only reads announces no input at all.
    #[test]
    fn an_ioctl_with_no_input_has_no_data() {
        let req = decode(b"", 128).expect("decodes");

        assert_eq!(req.in_len(), 0);
        assert!(req.data().is_empty());
        assert_eq!(req.out_len(), 128);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Ioctl::decode(request(Opcode::IOCTL, &[0u8; 16]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
