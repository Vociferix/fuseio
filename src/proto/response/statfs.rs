use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};

#[repr(C)]
#[derive(Debug)]
pub struct StatFs {
    blocks: u64,
    bfree: u64,
    bavail: u64,
    files: u64,
    ffree: u64,
    bsize: u32,
    namelen: u32,
    frsize: u32,
    _unused: u32,
}

#[repr(C)]
struct StatFsCompat {
    blocks: u64,
    bfree: u64,
    bavail: u64,
    files: u64,
    ffree: u64,
    bsize: u32,
    namelen: u32,
}

impl StatFs {
    pub const fn new() -> Self {
        StatFs {
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: 0,
            ffree: 0,
            bsize: 4096,
            namelen: 255,
            frsize: 0,
            _unused: 0,
        }
    }

    pub fn data_blocks(mut self, total: u64) -> Self {
        self.blocks = total;
        self
    }

    pub fn free_data_blocks(mut self, total_free: u64) -> Self {
        self.bfree = total_free;
        self
    }

    pub fn available_data_blocks(mut self, total_avail: u64) -> Self {
        self.bavail = total_avail;
        self
    }

    pub fn inodes(mut self, total_inodes: u64) -> Self {
        self.files = total_inodes;
        self
    }

    pub fn free_inodes(mut self, total_free: u64) -> Self {
        self.ffree = total_free;
        self
    }

    pub fn block_size(mut self, size: usize) -> Self {
        self.bsize = u32::try_from(size).unwrap_or(const { (u32::MAX >> 1) + 1 });
        self
    }

    pub fn max_name_len(mut self, max_len: usize) -> Self {
        self.namelen = u32::try_from(max_len).unwrap_or(u32::MAX);
        self
    }

    pub fn fragment_size(mut self, frag_size: usize) -> Self {
        self.frsize = u32::try_from(frag_size).unwrap_or(u32::MAX);
        self
    }
}

impl Default for StatFs {
    fn default() -> Self {
        const { Self::new() }
    }
}

impl EncodeResp for StatFs {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            statfs: StatFs,
            _unused: [u32; 6],
        }

        #[repr(C)]
        struct OutCompat {
            hdr: RawHeader,
            statfs: StatFsCompat,
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

        let len = if cfg.minor_ver < 4 {
            const { std::mem::size_of::<OutCompat>() as u32 }
        } else {
            const { std::mem::size_of::<Out>() as u32 }
        };

        Ok(Out {
            hdr: RawHeader { len, err: 0, id },
            statfs: self,
            _unused: [0; 6],
        })
    }
}
