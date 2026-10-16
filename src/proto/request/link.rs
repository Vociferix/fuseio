use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct Link {
    parent: Ino,
    ino: Ino,
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    src: u64,
}

impl Link {
    /// The directory the new entry goes in.
    pub fn parent(&self) -> Ino {
        self.parent
    }

    /// The file gaining another name.
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl Link {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(src) = Ino::from_raw(raw.src) else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
            buf.try_into_view(std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + name_len))
        } else {
            buf.try_into_view(std::mem::size_of::<Raw>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            parent: ino,
            ino: src,
            buf,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    fn body(src: u64, name: &[u8]) -> Vec<u8> {
        let mut body = src.to_ne_bytes().to_vec();
        body.extend_from_slice(name);
        body
    }

    fn decode(src: u64, name: &[u8]) -> Result<Link> {
        Link::decode(request(Opcode::LINK, &body(src, name)), root(), latest())
    }

    // The new name goes in the directory the header names; the body says which
    // inode is gaining it.
    #[test]
    fn the_body_names_the_inode_and_the_header_the_directory() {
        let req = decode(9, b"second-name\0").expect("decodes");

        assert_eq!(req.ino().as_raw(), 9);
        assert_eq!(req.parent(), root().unwrap());
        assert_eq!(req.name(), "second-name");
    }

    // Inode zero names nothing, so a link to it is not a link.
    #[test]
    fn a_link_to_inode_zero_is_refused() {
        assert_eq!(decode(0, b"x\0").unwrap_err(), Error::EINVAL);
    }

    #[test]
    fn a_body_too_short_for_the_source_inode_is_refused() {
        let err = Link::decode(request(Opcode::LINK, &[0u8; 4]), root(), latest()).unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err =
            Link::decode(request(Opcode::LINK, &body(9, b"x\0")), None, latest()).unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
