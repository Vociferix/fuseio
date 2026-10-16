use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RmDir {
    parent: Ino,
    buf: BufView,
}

impl RmDir {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl RmDir {
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

    #[test]
    fn the_name_stops_at_the_terminator() {
        let req =
            RmDir::decode(request(Opcode::RMDIR, b"gone\0"), root(), latest()).expect("decodes");

        assert_eq!(req.parent(), root().unwrap());
        assert_eq!(req.name(), "gone");
    }

    #[test]
    fn padding_after_the_terminator_is_not_part_of_the_name() {
        let req =
            RmDir::decode(request(Opcode::RMDIR, b"a\0\0\0"), root(), latest()).expect("decodes");

        assert_eq!(req.name(), "a");
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = RmDir::decode(request(Opcode::RMDIR, b"x\0"), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
