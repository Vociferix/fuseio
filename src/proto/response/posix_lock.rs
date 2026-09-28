use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{FileRange, LockKind, Tgid};

#[repr(C)]
#[derive(Debug)]
pub struct PosixLock {
    start: u64,
    end: u64,
    kind: u32,
    pid: i32,
}

impl PosixLock {
    // `F_*LCK` are `c_int` on Linux and `c_short` elsewhere.
    #[allow(clippy::useless_conversion)]
    pub fn new(kind: LockKind) -> Self {
        Self {
            start: 0,
            end: FileRange::OFFSET_MAX,
            // TODO(e2e): assumes host-native F_*LCK values; verify once end-to-end
            // tests can be done.
            kind: i32::from(match kind {
                LockKind::Unlock => nix::libc::F_UNLCK,
                LockKind::Read => nix::libc::F_RDLCK,
                LockKind::Write => nix::libc::F_WRLCK,
            })
            .cast_unsigned(),
            pid: 0,
        }
    }

    pub fn range<R>(mut self, range: R) -> Self
    where
        R: Into<FileRange>,
    {
        let range = range.into();

        self.start = range.start_offset();
        self.end = range.wire_end();

        self
    }

    /// Sets the thread group holding the conflicting lock.
    pub fn tgid(mut self, tgid: Tgid) -> Self {
        self.pid = tgid.as_raw();
        self
    }
}

impl EncodeResp for PosixLock {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            lock: PosixLock,
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
            lock: self,
        })
    }
}
