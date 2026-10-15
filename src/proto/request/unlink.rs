use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct Unlink {
    parent: Ino,
    buf: BufView,
}

impl Unlink {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl Unlink {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, name)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
            buf.try_into_view(
                std::mem::size_of::<RawHeader>()..(std::mem::size_of::<RawHeader>() + name_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawHeader>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { parent: ino, buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    fn decode(body: &[u8]) -> Result<Unlink> {
        Unlink::decode(request(Opcode::UNLINK, body), root(), latest())
    }

    #[test]
    fn the_name_stops_at_the_terminator() {
        let req = decode(b"gone.txt\0").expect("decodes");

        assert_eq!(req.name(), "gone.txt");
        assert_eq!(req.parent(), root().unwrap());
    }

    #[test]
    fn padding_after_the_terminator_is_not_part_of_the_name() {
        let req = decode(b"a\0\0\0\0").expect("decodes");

        assert_eq!(req.name(), "a");
    }

    // A name the kernel failed to terminate is still the whole of what arrived
    // rather than a read past the end of it.
    #[test]
    fn an_unterminated_name_is_taken_as_what_arrived() {
        let req = decode(b"no-terminator").expect("decodes");

        assert_eq!(req.name(), "no-terminator");
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = Unlink::decode(request(Opcode::UNLINK, b"x\0"), None, latest()).unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
