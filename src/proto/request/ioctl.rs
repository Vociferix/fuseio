use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, IoctlCmd, IoctlFlags};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Ioctl {
    ino: Ino,
    fh: FileHandle,
    flags: IoctlFlags,
    cmd: IoctlCmd,
    arg: u64,
    in_len: usize,
    out_len: usize,
    buf: Buf,
}

#[repr(C)]
pub struct Raw {
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
        &self.buf[const { HDR_LEN + std::mem::size_of::<Raw>() }..]
    }
}

impl Ioctl {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            flags: raw.flags,
            cmd: raw.cmd,
            arg: raw.arg,
            in_len: raw.in_size as usize,
            out_len: raw.out_size as usize,
            buf,
        })
    }
}
