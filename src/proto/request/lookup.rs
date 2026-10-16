use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct Lookup {
    parent: Ino,
    buf: BufView,
}

impl Lookup {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl Lookup {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, name)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let name_len = memchr::memchr(0, name).unwrap_or_else(|| name.len());

        let Some(buf) = buf.try_into_view(
            std::mem::size_of::<RawHeader>()..(std::mem::size_of::<RawHeader>() + name_len),
        ) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { parent: ino, buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{cfg, latest, request, request_on, root};

    fn decode(body: &[u8]) -> Result<Lookup> {
        Lookup::decode(request(Opcode::LOOKUP, body), root(), latest())
    }

    // The kernel terminates the name; the terminator is not part of it.
    #[test]
    fn the_name_stops_at_the_terminator() {
        let req = decode(b"hello.txt\0").expect("decodes");

        assert_eq!(req.name(), "hello.txt");
        assert_eq!(req.parent(), root().unwrap());
    }

    // The kernel sizes requests in whole words, so a name can be followed by
    // padding that is not part of it.
    #[test]
    fn padding_after_the_terminator_is_not_part_of_the_name() {
        let req = decode(b"a\0\0\0\0\0\0\0").expect("decodes");

        assert_eq!(req.name(), "a");
    }

    #[test]
    fn the_parent_is_whichever_inode_the_header_named() {
        let req = Lookup::decode(
            request_on(Opcode::LOOKUP, 42, b"x\0"),
            Ino::from_raw(42),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.parent().as_raw(), 42);
    }

    // Inode zero is not a thing the protocol names, so a request carrying it is
    // a request about nothing.
    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err =
            Lookup::decode(request_on(Opcode::LOOKUP, 0, b"x\0"), None, latest()).unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }

    #[test]
    fn a_body_shorter_than_the_header_is_refused() {
        let mut buf = crate::buf::BufPool::new().checkout_with_capacity(8);
        buf.extend_from_slice(b"short");

        assert_eq!(
            Lookup::decode(buf, root(), latest()).unwrap_err(),
            Error::EPROTO
        );
    }

    // The version has no bearing on this layout, so an old peer is served the
    // same way.
    #[test]
    fn the_layout_does_not_depend_on_the_version() {
        let req =
            Lookup::decode(request(Opcode::LOOKUP, b"old\0"), root(), cfg(19)).expect("decodes");

        assert_eq!(req.name(), "old");
    }
}
