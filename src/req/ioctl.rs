use super::Req;
use crate::proto::request::Ioctl;
use crate::types::{Abi, FileHandle, Ino, IoctlCmd, IoctlFlags};

#[derive(Debug)]
pub struct IoctlReq<C> {
    req: Req<C>,
    ioctl: Ioctl,
}

impl<C> IoctlReq<C> {
    pub(crate) fn new(req: Req<C>, ioctl: Ioctl) -> Self {
        Self { req, ioctl }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
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

    pub fn is_directory(&self) -> bool {
        self.ioctl.flags().contains(IoctlFlags::DIRECTORY)
    }

    pub fn is_unrestricted(&self) -> bool {
        self.ioctl.flags().contains(IoctlFlags::UNRESTRICTED)
    }

    pub fn command(&self) -> IoctlCmd {
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

    pub fn cast_data<T>(&self) -> Result<&T, bytemuck::PodCastError>
    where
        T: bytemuck::AnyBitPattern,
    {
        const {
            assert!(
                std::mem::align_of::<T>() <= 8,
                "IoctlReq::cast_data<T> requires T has an alignment of 8 or less - use IoctlReq::decode_data<T> instead",
            )
        };

        bytemuck::try_from_bytes(self.data())
    }

    pub fn decode_data<T>(&self) -> Result<T, bytemuck::PodCastError>
    where
        T: bytemuck::AnyBitPattern,
    {
        bytemuck::try_pod_read_unaligned(self.data())
    }
}

impl<C> std::ops::Deref for IoctlReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
