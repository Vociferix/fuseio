use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetVolName {
    buf: BufView,
}

impl SetVolName {
    pub fn volume_name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl SetVolName {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, name)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
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

        Ok(Self { buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request};

    fn decode(body: &[u8]) -> Result<SetVolName> {
        SetVolName::decode(request(Opcode::SETVOLNAME, body), None, latest())
    }

    // The body is the volume's new name, terminated, and nothing else. macOS
    // is the only sender, but the layout is the same everywhere, so this
    // decodes on any host.
    #[test]
    fn the_name_stops_at_the_terminator() {
        let req = decode(b"Macintosh HD\0").expect("decodes");

        assert_eq!(req.volume_name(), "Macintosh HD");
    }

    #[test]
    fn padding_after_the_terminator_is_not_part_of_the_name() {
        let req = decode(b"disk\0\0\0\0").expect("decodes");

        assert_eq!(req.volume_name(), "disk");
    }

    // The name runs to the end of the request, so one the kext left
    // unterminated is still unambiguous.
    #[test]
    fn an_unterminated_name_is_taken_as_what_arrived() {
        let req = decode(b"no-terminator").expect("decodes");

        assert_eq!(req.volume_name(), "no-terminator");
    }

    // Renaming a volume to nothing is still a request the filesystem gets to
    // answer, rather than one to refuse here.
    #[test]
    fn an_empty_name_decodes() {
        let req = decode(b"\0").expect("decodes");

        assert_eq!(req.volume_name(), "");
    }

    // A volume name is a user-facing string and need not be UTF-8.
    #[test]
    fn a_name_that_is_not_utf8_survives() {
        use std::os::unix::ffi::OsStrExt;

        let req = decode(b"a\xffb\0").expect("decodes");

        assert_eq!(req.volume_name().as_bytes(), b"a\xffb");
    }

    // The request names a volume rather than an inode, so the header's nodeid
    // plays no part.
    #[test]
    fn it_does_not_need_an_inode() {
        assert_eq!(decode(b"vol\0").unwrap().volume_name(), "vol");
    }
}
