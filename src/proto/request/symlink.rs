use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug)]
pub struct Symlink {
    parent: Ino,
    buf: BufView,
    name_len: usize,
}

impl Symlink {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[..self.name_len])
    }

    pub fn link(&self) -> &Path {
        Path::new(OsStr::from_bytes(&self.buf[(self.name_len + 1)..]))
    }
}

impl Symlink {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, data)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(name_len) = memchr::memchr(0, data) else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(link_len) = memchr::memchr(0, &data[(name_len + 1)..]) {
            buf.try_into_view(
                std::mem::size_of::<RawHeader>()
                    ..(std::mem::size_of::<RawHeader>() + name_len + 1 + link_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawHeader>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            parent: ino,
            buf,
            name_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    fn decode(body: &[u8]) -> Result<Symlink> {
        Symlink::decode(request(Opcode::SYMLINK, body), root(), latest())
    }

    // Two terminated strings back to back: the name of the link, then what it
    // points at.
    #[test]
    fn the_name_and_the_target_are_two_terminated_strings() {
        let req = decode(b"link\0../target\0").expect("decodes");

        assert_eq!(req.name(), "link");
        assert_eq!(req.link(), std::path::Path::new("../target"));
        assert_eq!(req.parent(), root().unwrap());
    }

    // Without the first terminator there is no way to tell where the name ends
    // and the target begins, so there is nothing to guess at.
    #[test]
    fn a_name_with_no_terminator_is_refused() {
        assert_eq!(decode(b"just-a-name").unwrap_err(), Error::EPROTO);
    }

    // The target's terminator is the last byte of the request, so a kernel that
    // left it off still leaves the target unambiguous.
    #[test]
    fn an_unterminated_target_is_taken_as_the_rest() {
        let req = decode(b"link\0/target").expect("decodes");

        assert_eq!(req.name(), "link");
        assert_eq!(req.link(), std::path::Path::new("/target"));
    }

    #[test]
    fn a_target_may_be_empty() {
        let req = decode(b"link\0\0").expect("decodes");

        assert_eq!(req.name(), "link");
        assert_eq!(req.link(), std::path::Path::new(""));
    }

    #[test]
    fn padding_after_the_target_is_not_part_of_it() {
        let req = decode(b"link\0t\0\0\0\0").expect("decodes");

        assert_eq!(req.link(), std::path::Path::new("t"));
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = Symlink::decode(request(Opcode::SYMLINK, b"a\0b\0"), None, latest()).unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
