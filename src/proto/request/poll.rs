use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, PollFlags};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Poll {
    ino: Ino,
    fh: FileHandle,
    poll_handle: Option<u64>,
    interests: PollFlags,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct RawFlags: u32 {
        const SCHEDULE_NOTIFY = 1 << 0;
    }
}

#[repr(C)]
struct Raw {
    fh: u64,
    kh: u64,
    flags: RawFlags,
    events: u32,
}

impl Poll {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn poll_handle(&self) -> Option<u64> {
        self.poll_handle
    }

    pub fn interests(&self) -> PollFlags {
        self.interests
    }
}

impl Poll {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EINVAL);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            poll_handle: raw
                .flags
                .contains(RawFlags::SCHEDULE_NOTIFY)
                .then_some(raw.kh),
            // TODO(e2e): assumes host-native poll event values; verify once
            // end-to-end tests can be done.
            interests: PollFlags::from_bits_retain((raw.events as u16).cast_signed()),
        })
    }
}
