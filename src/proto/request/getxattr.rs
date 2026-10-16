use super::{Cfg, Ino};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattr {
    ino: Ino,
    buf: BufView,
    len: usize,
    #[cfg(target_os = "macos")]
    offset: usize,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
pub(super) struct Raw {
    _hdr: super::RawHeader,
    pub(super) size: u32,
    _unused0: u32,
    #[cfg(target_os = "macos")]
    pub(super) position: u32,
    #[cfg(target_os = "macos")]
    _unused1: u32,
}

impl GetXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &OsStr {
        self.buf.as_os_str()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn offset(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.offset
        }

        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }
}

impl GetXattr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let len = raw.size as usize;

        // Copied out before `buf` is moved below: `raw` borrows from it.
        #[cfg(target_os = "macos")]
        let offset = raw.position as usize;

        let Some(buf) = (if let Some(key_len) = memchr::memchr(0, name) {
            buf.try_into_view(std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + key_len))
        } else {
            buf.try_into_view(std::mem::size_of::<Raw>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            ino,
            buf,
            len,
            #[cfg(target_os = "macos")]
            offset,
        })
    }
}

/// The body length a request of this kind needs, which differs by
/// platform where the layout does.
#[cfg(test)]
pub(super) const RAW_LEN: usize =
    std::mem::size_of::<Raw>() - std::mem::size_of::<super::RawHeader>();

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, request_on, root};

    /// `fuse_getxattr_in`: the caller's buffer size, then the key.
    fn body(size: u32, key: &[u8]) -> Vec<u8> {
        let mut body = size.to_ne_bytes().to_vec();
        body.extend_from_slice(&0u32.to_ne_bytes()); // padding
        #[cfg(target_os = "macos")]
        {
            body.extend_from_slice(&0u32.to_ne_bytes()); // position
            body.extend_from_slice(&0u32.to_ne_bytes()); // padding
        }
        body.extend_from_slice(key);
        body
    }

    fn decode(size: u32, key: &[u8]) -> Result<GetXattr> {
        GetXattr::decode(
            request(Opcode::GETXATTR, &body(size, key)),
            root(),
            latest(),
        )
    }

    // `size` is how much room the caller has, and the key names the attribute.
    // Both come from the body, after the request header.
    #[test]
    fn the_size_and_the_key_come_from_the_body() {
        let req = decode(4096, b"user.comment\0").expect("decodes");

        assert_eq!(req.len(), 4096);
        assert_eq!(req.key(), "user.comment");
        assert_eq!(req.ino(), root().unwrap());
    }

    // A size of zero asks how large the value is rather than for the value, so
    // it has to survive as a zero.
    #[test]
    fn a_size_of_zero_asks_for_the_length() {
        let req = decode(0, b"user.comment\0").expect("decodes");

        assert_eq!(req.len(), 0);
        assert_eq!(req.key(), "user.comment");
    }

    #[test]
    fn padding_after_the_key_is_not_part_of_it() {
        let req = decode(64, b"user.a\0\0\0").expect("decodes");

        assert_eq!(req.key(), "user.a");
    }

    #[test]
    fn the_inode_is_whichever_the_header_named() {
        let req = GetXattr::decode(
            request_on(Opcode::GETXATTR, 99, &body(8, b"user.a\0")),
            Ino::from_raw(99),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.ino().as_raw(), 99);
    }

    #[test]
    fn a_body_too_short_for_the_size_field_is_refused() {
        let err = GetXattr::decode(request(Opcode::GETXATTR, &[0u8; 4]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = GetXattr::decode(
            request(Opcode::GETXATTR, &body(8, b"user.a\0")),
            None,
            latest(),
        )
        .map(|_| ())
        .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
