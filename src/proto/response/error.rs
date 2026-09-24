use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};

impl EncodeResp for crate::Error {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        Ok(self.into_reply(id))
    }
}

impl crate::Error {
    pub(crate) fn into_reply(self, id: u64) -> impl IntoIoBuf {
        let errno = self.wire_errno();

        RawHeader {
            len: const { std::mem::size_of::<RawHeader>() as u32 },
            // TODO(e2e): assumes host-native errno values; verify once end-to-end
            // tests can be done.
            err: -errno,
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Error;
    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn reply(err: Error) -> (u32, i32, u64) {
        let buf = err.encode(7, cfg()).unwrap().into_io_buf();
        let bytes: Vec<u8> = buf.iter_slice().flatten().copied().collect();

        assert_eq!(bytes.len(), 16);

        (
            u32::from_ne_bytes(bytes[0..4].try_into().unwrap()),
            i32::from_ne_bytes(bytes[4..8].try_into().unwrap()),
            u64::from_ne_bytes(bytes[8..16].try_into().unwrap()),
        )
    }

    #[test]
    fn an_error_reply_is_a_bare_header() {
        let (len, err, id) = reply(Error::ENOENT);

        assert_eq!(len, 16);
        assert_eq!(err, -Error::ENOENT.raw_os_error());
        assert_eq!(id, 7);
    }

    #[test]
    fn the_errno_is_negated() {
        let (_, err, _) = reply(Error::EIO);

        assert!(err < 0);
        assert_eq!(-err, Error::EIO.raw_os_error());
    }

    #[test]
    fn an_errno_no_kernel_accepts_becomes_eio() {
        let too_big = Error::from(std::num::NonZeroI32::new(9999).unwrap());
        let negative = Error::from(std::num::NonZeroI32::new(-5).unwrap());

        assert_eq!(reply(too_big).1, -Error::EIO.raw_os_error());
        assert_eq!(reply(negative).1, -Error::EIO.raw_os_error());
    }

    #[test]
    fn the_error_stays_within_what_kernels_accept() {
        let (_, err, _) = reply(Error::from(std::num::NonZeroI32::new(511).unwrap()));

        assert!(err > -512 && err < 0);
    }
}
