use super::attr::InodeAttrsCompat;
use super::{Cfg, EncodeResp, InodeAttrs, IntoIoBuf, IoBuf, RawHeader};
use crate::types::Ino;

use std::time::Duration;

#[derive(Debug)]
#[repr(C)]
pub struct Entry {
    ino: u64,
    generation: u64,
    entry_valid: u64,
    attr_valid: u64,
    entry_valid_nsec: u32,
    attr_valid_nsec: u32,
    attr: InodeAttrs,
}

#[repr(C)]
pub struct EntryCompat {
    ino: u64,
    generation: u64,
    entry_valid: u64,
    attr_valid: u64,
    entry_valid_nsec: u32,
    attr_valid_nsec: u32,
    attr: InodeAttrsCompat,
}

impl Entry {
    pub fn new(ino: Ino) -> Self {
        Self {
            ino: ino.as_raw(),
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr: InodeAttrs::default(),
        }
    }

    pub fn not_found() -> Self {
        Self {
            ino: 0,
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr: InodeAttrs::default(),
        }
    }

    pub fn generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    pub fn entry_ttl(mut self, ttl: Duration) -> Self {
        self.entry_valid = ttl.as_secs();
        self.entry_valid_nsec = ttl.subsec_nanos();
        self
    }

    pub fn attrs_ttl(mut self, ttl: Duration) -> Self {
        self.attr_valid = ttl.as_secs();
        self.attr_valid_nsec = ttl.subsec_nanos();
        self
    }

    pub fn attrs(mut self, attrs: InodeAttrs) -> Self {
        self.attr = attrs;
        self
    }
}

impl EncodeResp for Entry {
    type Error = crate::Error;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            entry: Entry,
        }

        #[repr(C)]
        struct OutCompat {
            hdr: RawHeader,
            entry: EntryCompat,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                self.hdr.len as usize
            }
        }

        let len = match (self.ino, cfg.minor_ver) {
            (0, ..4) => return Err(crate::Error::ENOENT),
            (_, ..9) => std::mem::size_of::<OutCompat>(),
            _ => std::mem::size_of::<Out>(),
        };

        Ok(Out {
            hdr: RawHeader {
                len: len as u32,
                err: 0,
                id,
            },
            entry: self,
        })
    }
}
