use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Interrupt {
    id: u64,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    _hdr: RawHeader,
    id: u64,
}

impl Interrupt {
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Interrupt {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        Ok(Self { id: raw.id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, request_on, root};

    /// `fuse_interrupt_in { unique }`: which request to give up on.
    #[test]
    fn the_interrupted_request_comes_from_the_body() {
        let req = Interrupt::decode(
            request(Opcode::INTERRUPT, &0x1234_5678_9abcu64.to_ne_bytes()),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.id(), 0x1234_5678_9abc);
    }

    // An interrupt names a request rather than an inode, so the header's
    // nodeid has no part in it.
    #[test]
    fn it_does_not_need_an_inode() {
        let req = Interrupt::decode(
            request_on(Opcode::INTERRUPT, 0, &7u64.to_ne_bytes()),
            None,
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.id(), 7);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Interrupt::decode(request(Opcode::INTERRUPT, &[0u8; 4]), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
