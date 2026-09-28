use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::conn::Connection;
use crate::fs::types::PassthroughFd;
use crate::types::{FileHandle, OpenedFlags};

use std::os::fd::AsFd;

#[repr(C)]
#[derive(Debug)]
pub struct Opened {
    fh: u64,
    open_flags: u32,
    backing_id: u32,
}

const PASSTHROUGH: u32 = 1u32 << 7;

impl Opened {
    pub fn new(fh: FileHandle) -> Self {
        Self {
            fh: fh.0,
            open_flags: 0,
            backing_id: 0,
        }
    }

    pub fn flags(mut self, flags: OpenedFlags) -> Self {
        self.open_flags = flags.bits();
        self
    }

    pub fn add_flags(mut self, flags: OpenedFlags) -> Self {
        self.open_flags |= flags.bits();
        self
    }

    pub fn passthrough<T: AsFd, C: Connection>(mut self, fd: &PassthroughFd<T, C>) -> Self {
        self.open_flags |= PASSTHROUGH;
        self.backing_id = PassthroughFd::backing_id(fd).0;
        self
    }
}

impl EncodeResp for Opened {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            open: Opened,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                const { std::mem::size_of::<Out>() }
            }
        }

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            open: self,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    const OPEN_FLAGS: usize = 16 + 8;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn encode(opened: Opened) -> Vec<u8> {
        let buf = opened.encode(7, cfg()).unwrap().into_io_buf();

        buf.iter_slice().flatten().copied().collect()
    }

    fn flags_of(opened: Opened) -> u32 {
        let bytes = encode(opened);

        u32::from_ne_bytes(bytes[OPEN_FLAGS..OPEN_FLAGS + 4].try_into().unwrap())
    }

    #[test]
    fn the_reply_carries_the_handle() {
        let bytes = encode(Opened::new(FileHandle(9)));

        assert_eq!(bytes.len(), 32);
        assert_eq!(u64::from_ne_bytes(bytes[16..24].try_into().unwrap()), 9);
    }

    #[test]
    fn the_flags_reach_the_wire() {
        let flags = flags_of(
            Opened::new(FileHandle(9)).flags(OpenedFlags::DIRECT_IO | OpenedFlags::NONSEEKABLE),
        );

        assert_eq!(flags, 1 | (1 << 2));
    }

    // macOS reads these; the other kernels ignore them.
    #[test]
    fn the_macos_purge_flags_reach_the_wire() {
        let flags = flags_of(
            Opened::new(FileHandle(9)).flags(OpenedFlags::PURGE_UBC | OpenedFlags::PURGE_ATTR),
        );

        assert_eq!(flags, (1 << 31) | (1 << 30));
    }

    #[test]
    fn added_flags_accumulate() {
        let flags = flags_of(
            Opened::new(FileHandle(9))
                .flags(OpenedFlags::DIRECT_IO)
                .add_flags(OpenedFlags::PURGE_UBC),
        );

        assert_eq!(flags, 1 | (1 << 31));
    }
}
