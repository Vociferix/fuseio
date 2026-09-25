use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, PollFlags};
use crate::{Error, Result, buf::Buf};

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
            return Err(Error::EPROTO);
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    const SCHEDULE_NOTIFY: u32 = 1 << 0;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn request(flags: u32) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(64);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&9u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&5u64.to_ne_bytes()); // kh
        buf.extend_from_slice(&flags.to_ne_bytes());
        buf.extend_from_slice(&(nix::libc::POLLIN as u32).to_ne_bytes());

        buf
    }

    #[test]
    fn the_body_decodes() {
        let req = Poll::decode(request(0), Ino::from_raw(1), cfg()).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert!(req.poll_handle().is_none());
        assert!(req.interests().contains(PollFlags::POLLIN));
    }

    #[test]
    fn a_handle_arrives_only_when_a_wakeup_is_wanted() {
        let req = Poll::decode(request(SCHEDULE_NOTIFY), Ino::from_raw(1), cfg()).unwrap();

        assert_eq!(req.poll_handle(), Some(5));
    }

    // A malformed request is a protocol error, as it is everywhere else.
    #[test]
    fn a_short_body_is_rejected() {
        let mut buf = BufPool::new().checkout_with_capacity(64);
        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&9u64.to_ne_bytes());

        assert_eq!(
            Poll::decode(buf, Ino::from_raw(1), cfg()).unwrap_err(),
            Error::EPROTO
        );
    }
}
