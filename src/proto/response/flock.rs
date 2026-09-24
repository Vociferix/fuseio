use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{FileRange, LockKind, Tgid};

#[derive(Debug)]
pub struct Flock {
    kind: LockKind,
    tgid: Option<Tgid>,
}

impl Flock {
    pub fn new(kind: LockKind) -> Self {
        Self { kind, tgid: None }
    }

    /// Sets the thread group holding the conflicting lock.
    pub fn tgid(mut self, tgid: Tgid) -> Self {
        self.tgid = Some(tgid);
        self
    }
}

impl From<LockKind> for Flock {
    fn from(kind: LockKind) -> Self {
        Self::new(kind)
    }
}

impl EncodeResp for Flock {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            start: u64,
            end: u64,
            kind: u32,
            pid: i32,
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

        // TODO(e2e): assumes host-native F_*LCK values; verify once end-to-end
        // tests can be done.
        let kind = match self.kind {
            LockKind::Unlock => nix::libc::F_UNLCK,
            LockKind::Read => nix::libc::F_RDLCK,
            LockKind::Write => nix::libc::F_WRLCK,
        };

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            start: 0,
            end: FileRange::OFFSET_MAX,
            kind: i32::from(kind).cast_unsigned(),
            pid: self.tgid.map_or(0, Tgid::as_raw),
        })
    }
}
