use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};

#[repr(C)]
#[derive(Debug)]
pub struct FsAttrs {
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
struct FsAttrsCompat {
    blocks: u64,
    bfree: u64,
    bavail: u64,
    files: u64,
    ffree: u64,
    bsize: u32,
    namelen: u32,
}

impl FsAttrs {
    pub const fn new() -> Self {
        FsAttrs {
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: 0,
            ffree: 0,
            bsize: 4096,
            namelen: 255,
            // Zero means "same as the block size", resolved when encoding.
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

    pub fn max_name_length(mut self, max_len: usize) -> Self {
        self.namelen = u32::try_from(max_len).unwrap_or(u32::MAX);
        self
    }

    /// Sets the fundamental block size, which defaults to the block size.
    ///
    /// Linux reports this separately, while FreeBSD and macOS report it as the
    /// filesystem's block size and ignore or repurpose
    /// [`block_size`](Self::block_size).
    pub fn fragment_size(mut self, frag_size: usize) -> Self {
        self.frsize = u32::try_from(frag_size).unwrap_or(u32::MAX);
        self
    }
}

impl Default for FsAttrs {
    fn default() -> Self {
        const { Self::new() }
    }
}

impl EncodeResp for FsAttrs {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            statfs: FsAttrs,
            _unused: [u32; 6],
        }

        #[repr(C)]
        struct OutCompat {
            hdr: RawHeader,
            statfs: FsAttrsCompat,
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

        let mut statfs = self;

        // FreeBSD reports this as the filesystem's block size and ignores
        // `bsize`, so leaving it zero would show the filesystem as empty there.
        // macOS substitutes its own default, and glibc's `statvfs` falls back the
        // same way.
        if statfs.frsize == 0 {
            statfs.frsize = statfs.bsize;
        }

        Ok(Out {
            hdr: RawHeader { len, err: 0, id },
            statfs,
            _unused: [0; 6],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    // `fuse_kstatfs` behind the 16-byte header.
    const BSIZE: usize = 16 + 40;
    const NAMELEN: usize = 16 + 44;
    const FRSIZE: usize = 16 + 48;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn encode(attrs: FsAttrs) -> Vec<u8> {
        let buf = attrs
            .encode(7, cfg(crate::handshake::MINOR_VER))
            .unwrap()
            .into_io_buf();

        buf.iter_slice().flatten().copied().collect()
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn the_defaults_describe_a_usable_filesystem() {
        let bytes = encode(FsAttrs::new());

        assert_eq!(u32_at(&bytes, BSIZE), 4096);
        assert_eq!(u32_at(&bytes, NAMELEN), 255);
        // FreeBSD reports this as the block size, so it can't be left zero.
        assert_eq!(u32_at(&bytes, FRSIZE), 4096);
    }

    #[test]
    fn the_fragment_size_follows_the_block_size() {
        let bytes = encode(FsAttrs::new().block_size(64 * 1024));

        assert_eq!(u32_at(&bytes, BSIZE), 64 * 1024);
        assert_eq!(u32_at(&bytes, FRSIZE), 64 * 1024);
    }

    #[test]
    fn an_explicit_fragment_size_is_kept_in_either_order() {
        let after = encode(FsAttrs::new().block_size(64 * 1024).fragment_size(512));
        let before = encode(FsAttrs::new().fragment_size(512).block_size(64 * 1024));

        for bytes in [after, before] {
            assert_eq!(u32_at(&bytes, BSIZE), 64 * 1024);
            assert_eq!(u32_at(&bytes, FRSIZE), 512);
        }
    }

    #[test]
    fn an_old_kernel_gets_the_shorter_reply() {
        let full = encode(FsAttrs::new());
        let compat = {
            let buf = FsAttrs::new().encode(7, cfg(3)).unwrap().into_io_buf();
            let bytes: Vec<u8> = buf.iter_slice().flatten().copied().collect();
            bytes
        };

        assert_eq!(full.len(), 16 + 80);
        assert_eq!(u32_at(&compat, 0), (16 + 48) as u32);
    }
}
