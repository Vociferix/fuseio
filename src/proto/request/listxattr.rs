use super::{Cfg, Ino};
use crate::{Error, Result, buf::Buf};

use super::getxattr::Raw;

#[derive(Debug)]
pub struct ListXattr {
    ino: Ino,
    len: usize,
}

impl ListXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl ListXattr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            len: raw.size as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_getxattr_in`, which `FUSE_LISTXATTR` reuses. No key follows it.
    fn body(size: u32) -> Vec<u8> {
        let mut body = size.to_ne_bytes().to_vec();
        body.extend_from_slice(&0u32.to_ne_bytes()); // padding
        #[cfg(target_os = "macos")]
        body.extend_from_slice(&[0u8; 8]); // position, padding
        body
    }

    fn decode(size: u32) -> Result<ListXattr> {
        ListXattr::decode(request(Opcode::LISTXATTR, &body(size)), root(), latest())
    }

    #[test]
    fn the_size_comes_from_the_body() {
        let req = decode(4096).expect("decodes");

        assert_eq!(req.len(), 4096);
        assert_eq!(req.ino(), root().unwrap());
    }

    // Zero asks how long the list is rather than for the list, so the two cases
    // have to stay distinguishable.
    #[test]
    fn a_size_of_zero_asks_for_the_length() {
        assert_eq!(decode(0).expect("decodes").len(), 0);
    }

    #[test]
    fn a_body_too_short_for_the_size_field_is_refused() {
        let err = ListXattr::decode(request(Opcode::LISTXATTR, &[0u8; 4]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = ListXattr::decode(request(Opcode::LISTXATTR, &body(8)), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
