use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct Poll {
    kh: u64,
}

impl Poll {
    pub fn new(kh: u64) -> Self {
        Self { kh }
    }
}

impl EncodeNotify for Poll {
    type Error = std::convert::Infallible;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            kh: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                code: NotifyCode::POLL,
                _zero: 0,
            },
            kh: self.kh,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode, u64_at};

    // `fuse_notify_poll_wakeup_out { kh }`, code 1.
    #[test]
    fn the_wakeup_handle_follows_the_header() {
        let bytes = encode(Poll::new(0x1234_5678_9abc));

        assert_notify_header(&bytes, 1);
        assert_eq!(bytes.len(), HDR + 8);
        assert_eq!(u64_at(&bytes, HDR), 0x1234_5678_9abc);
    }
}
