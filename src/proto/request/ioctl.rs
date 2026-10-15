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
