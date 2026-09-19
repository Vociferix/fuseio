use super::entry::EntryCompat;
use super::{Cfg, EncodeResp, Entry, InodeAttrs, IntoIoBuf, IoBuf, Opened, RawHeader};
use crate::buf::Vectored;
use crate::fs::types::PassthroughFd;
use crate::types::{FileHandle, Ino, OpenedFlags};

use std::os::fd::AsFd;
use std::time::Duration;

#[derive(Debug)]
pub struct Created {
    entry: Entry,
    open: Opened,
}

impl Created {
    pub fn new(ino: Ino, fh: FileHandle) -> Self {
        Self {
            entry: Entry::new(ino),
            open: Opened::new(fh),
        }
    }

    pub fn from_parts(entry: Entry, open: Opened) -> Self {
        Self { entry, open }
    }

    pub fn generation(self, generation: u64) -> Self {
        Self {
            entry: self.entry.generation(generation),
            open: self.open,
        }
    }

    pub fn entry_ttl(self, ttl: Duration) -> Self {
        Self {
            entry: self.entry.entry_ttl(ttl),
            open: self.open,
        }
    }

    pub fn attr_ttl(self, ttl: Duration) -> Self {
        Self {
            entry: self.entry.entry_ttl(ttl),
            open: self.open,
        }
    }

    pub fn attrs(self, attrs: InodeAttrs) -> Self {
        Self {
            entry: self.entry.attrs(attrs),
            open: self.open,
        }
    }

    pub fn flags(self, flags: OpenedFlags) -> Self {
        Self {
            entry: self.entry,
            open: self.open.flags(flags),
        }
    }

    pub fn add_flags(self, flags: OpenedFlags) -> Self {
        Self {
            entry: self.entry,
            open: self.open.add_flags(flags),
        }
    }

    pub fn passthrough<T: AsFd>(self, fd: &PassthroughFd<T>) -> Self {
        Self {
            entry: self.entry,
            open: self.open.passthrough(fd),
        }
    }
}

impl From<(Entry, Opened)> for Created {
    fn from((entry, open): (Entry, Opened)) -> Self {
        Self::from_parts(entry, open)
    }
}

impl EncodeResp for Created {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        let Self { entry, open } = self;

        #[repr(transparent)]
        struct OpenedOut(Opened);

        #[repr(C)]
        struct EntryOut {
            hdr: RawHeader,
            entry: Entry,
        }

        #[repr(C)]
        struct EntryOutCompat {
            hdr: RawHeader,
            entry: EntryCompat,
        }

        impl IoBuf for OpenedOut {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                const { std::mem::size_of::<OpenedOut>() }
            }
        }

        impl IoBuf for EntryOut {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                self.hdr.len as usize - std::mem::size_of::<OpenedOut>()
            }
        }

        let len = if cfg.minor_ver < 9 {
            const { (std::mem::size_of::<EntryOutCompat>() + std::mem::size_of::<OpenedOut>()) as u32 }
        } else {
            const { (std::mem::size_of::<EntryOut>() + std::mem::size_of::<OpenedOut>()) as u32 }
        };

        Ok(Vectored((
            EntryOut {
                hdr: RawHeader { len, err: 0, id },
                entry,
            },
            (OpenedOut(open),),
        )))
    }
}
