use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Forget {
    ino: Ino,
    nlookup: u64,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
struct Raw {
    _hdr: RawHeader,
    nlookup: u64,
}

impl Forget {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn nlookup(&self) -> u64 {
        self.nlookup
    }
}

impl Forget {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            nlookup: raw.nlookup,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, request_on, root};

    /// `fuse_forget_in { nlookup }`.
    #[test]
    fn the_lookup_count_comes_from_the_body_and_the_inode_from_the_header() {
        let req = Forget::decode(
            request_on(Opcode::FORGET, 9, &7u64.to_ne_bytes()),
            Ino::from_raw(9),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.ino().as_raw(), 9);
        assert_eq!(req.nlookup(), 7);
    }

    // The kernel coalesces lookups, so a single forget can drop a great many.
    #[test]
    fn a_large_count_survives_intact() {
        let req = Forget::decode(
            request(Opcode::FORGET, &u64::MAX.to_ne_bytes()),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.nlookup(), u64::MAX);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Forget::decode(request(Opcode::FORGET, &[0u8; 4]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
