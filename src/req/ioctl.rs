use super::Req;
use crate::proto::request::Ioctl;
use crate::types::{Abi, FileHandle, Ino, IoctlFlags};

#[derive(Debug)]
pub struct IoctlReq {
    req: Req,
    ioctl: Ioctl,
}

impl IoctlReq {
    pub(crate) fn new(req: Req, ioctl: Ioctl) -> Self {
        Self { req, ioctl }
    }

    pub fn ino(&self) -> Ino {
        self.ioctl.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.ioctl.file_handle()
    }

    pub fn abi(&self) -> Abi {
        const ABI_32BIT_ON_64BIT: u32 = IoctlFlags::ABI_32BIT_ON_64BIT.bits();
        const ABI_32BIT: u32 = IoctlFlags::ABI_32BIT.bits();
        const ABI_X32: u32 = IoctlFlags::ABI_X32.bits();

        const MASK: u32 = ABI_32BIT_ON_64BIT | ABI_32BIT | ABI_X32;

        match self.ioctl.flags().bits() & MASK {
            ABI_32BIT_ON_64BIT => Abi::Abi32BitOn64Bit,
            ABI_32BIT => Abi::Abi32Bit,
            ABI_X32 => Abi::AbiX32,
            _ => Abi::Native,
        }
    }

    pub fn directory(&self) -> bool {
        self.ioctl.flags().contains(IoctlFlags::DIRECTORY)
    }

    pub fn unrestricted(&self) -> bool {
        self.ioctl.flags().contains(IoctlFlags::UNRESTRICTED)
    }

    pub fn command(&self) -> u32 {
        self.ioctl.command()
    }

    pub fn arg(&self) -> u64 {
        self.ioctl.arg()
    }

    pub fn in_len(&self) -> usize {
        self.ioctl.in_len()
    }

    pub fn out_len(&self) -> usize {
        self.ioctl.out_len()
    }

    pub fn data(&self) -> &[u8] {
        self.ioctl.data()
    }
}

impl std::ops::Deref for IoctlReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
