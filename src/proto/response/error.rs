use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};

impl EncodeResp for crate::Error {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        Ok(self.into_reply(id))
    }
}

impl crate::Error {
    // TODO: Linux rejects a reply whose error isn't in -511..=-1 (or 0) before
    // looking up the request, so the request is never completed and the caller
    // hangs. `Error` accepts any non-zero i32 (e.g. negative values), so map
    // anything outside 1..512 to EIO/ERANGE and log it, as libfuse does.
    pub(crate) fn into_reply(self, id: u64) -> impl IntoIoBuf {
        RawHeader {
            len: const { std::mem::size_of::<RawHeader>() as u32 },
            // TODO(e2e): assumes host-native errno values; verify once end-to-end
            // tests can be done.
            err: -self.raw_os_error(),
            id,
        }
    }
}
