use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveXattr {
    ino: Ino,
    buf: BufView,
}

impl RemoveXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl RemoveXattr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, key)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(key_len) = memchr::memchr(0, key) {
            buf.try_into_view(
                std::mem::size_of::<RawHeader>()..(std::mem::size_of::<RawHeader>() + key_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawHeader>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { ino, buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    // The body is the key alone, terminated.
    #[test]
    fn the_key_stops_at_the_terminator() {
        let req = RemoveXattr::decode(
            request(Opcode::REMOVEXATTR, b"user.gone\0"),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.key(), "user.gone");
    }

    #[test]
    fn padding_after_the_key_is_not_part_of_it() {
        let req = RemoveXattr::decode(
            request(Opcode::REMOVEXATTR, b"user.a\0\0\0"),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.key(), "user.a");
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = RemoveXattr::decode(request(Opcode::REMOVEXATTR, b"user.a\0"), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
