use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;
use crate::types::PollFlags;

#[derive(Debug)]
pub struct Poll {
    ready: PollFlags,
}

impl Poll {
    pub const fn new(ready: PollFlags) -> Self {
        Self { ready }
    }
}

impl From<PollFlags> for Poll {
    fn from(ready: PollFlags) -> Self {
        Self::new(ready)
    }
}

impl EncodeResp for Poll {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            ready: u32,
            _unused: u32,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            ready: self.ready.bits().cast_unsigned().into(),
            _unused: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::response::test_support::{HDR, assert_reply_header, encode, u32_at};

    // `fuse_poll_out { revents, padding }`. The padding is part of the reply's
    // length, so it has to be sent even though it carries nothing.
    #[test]
    fn the_ready_events_follow_the_header_with_their_padding() {
        let bytes = encode(Poll::new(PollFlags::POLLIN | PollFlags::POLLOUT), 3);

        assert_reply_header(&bytes, 3);
        assert_eq!(bytes.len(), HDR + 8);
        assert_eq!(
            u32_at(&bytes, HDR),
            (PollFlags::POLLIN | PollFlags::POLLOUT).bits() as u32
        );
        assert_eq!(u32_at(&bytes, HDR + 4), 0, "the padding must be zero");
    }

    #[test]
    fn nothing_ready_is_a_reply_of_no_events() {
        let bytes = encode(Poll::new(PollFlags::empty()), 3);

        assert_eq!(u32_at(&bytes, HDR), 0);
    }
}
