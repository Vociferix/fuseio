use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;
use crate::types::{FileRange, LockKind, Tgid};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
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
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            lock: PosixLock,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            lock: self,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::response::test_support::{HDR, assert_reply_header, encode, u32_at, u64_at};

    const START: usize = HDR;
    const END: usize = HDR + 8;
    const KIND: usize = HDR + 16;
    const PID: usize = HDR + 20;

    // `fuse_lk_out { lk: fuse_file_lock { start, end, type, pid } }`.
    #[test]
    fn the_lock_is_the_size_the_kernel_expects() {
        let bytes = encode(PosixLock::new(LockKind::Unlock), 9);

        assert_reply_header(&bytes, 9);
        assert_eq!(bytes.len(), HDR + 24);
    }

    // The kernel turns the reply back into a `struct flock`, where the end is
    // the last byte of the range rather than one past it.
    #[test]
    fn a_range_is_reported_by_its_last_byte() {
        let bytes = encode(PosixLock::new(LockKind::Write).range(100..200), 1);

        assert_eq!(u64_at(&bytes, START), 100);
        assert_eq!(u64_at(&bytes, END), 199, "the end should be inclusive");
    }

    // A lock with no end runs to the largest offset the kernel represents,
    // which is what it reads as "to the end of the file".
    #[test]
    fn a_lock_with_no_end_runs_to_the_maximum_offset() {
        let bytes = encode(PosixLock::new(LockKind::Read).range(4096..), 1);

        assert_eq!(u64_at(&bytes, START), 4096);
        assert_eq!(u64_at(&bytes, END), crate::types::FileRange::OFFSET_MAX);
    }

    // A fresh reply describes the whole file, which is what an unlocked answer
    // means.
    #[test]
    fn an_unlocked_reply_covers_the_whole_file() {
        let bytes = encode(PosixLock::new(LockKind::Unlock), 1);

        assert_eq!(u64_at(&bytes, START), 0);
        assert_eq!(u64_at(&bytes, END), crate::types::FileRange::OFFSET_MAX);
        assert_eq!(u32_at(&bytes, KIND), nix::libc::F_UNLCK as u32);
    }

    #[test]
    fn each_kind_reaches_the_wire_as_its_own_value() {
        for (kind, expected) in [
            (LockKind::Unlock, nix::libc::F_UNLCK),
            (LockKind::Read, nix::libc::F_RDLCK),
            (LockKind::Write, nix::libc::F_WRLCK),
        ] {
            let bytes = encode(PosixLock::new(kind), 1);

            assert_eq!(u32_at(&bytes, KIND), expected as u32, "{kind:?}");
        }
    }

    // The holder's thread group is what the caller sees as `l_pid`.
    #[test]
    fn the_holder_is_reported() {
        let bytes = encode(
            PosixLock::new(LockKind::Write).tgid(Tgid::from_raw(4242).unwrap()),
            1,
        );

        assert_eq!(u32_at(&bytes, PID), 4242);
    }
}
