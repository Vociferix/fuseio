//! Scaffolding for reply-encoder tests: replies read back as the bytes the
//! kernel would see.
//!
//! Offsets here come from `struct fuse_out_header` and the `fuse_*_out` bodies
//! in the kernel's `fuse.h`, not from the types being tested, so a test fails
//! when the two disagree.

use super::{Cfg, EncodeResp};
use crate::buf::IntoIoBuf;

use compio::buf::IoVectoredBuf;

/// `struct fuse_out_header`: `len`, `error`, `unique`.
pub(super) const HDR: usize = 16;

pub(super) fn cfg_at(minor_ver: u32) -> Cfg {
    Cfg {
        minor_ver,
        ..Cfg::default()
    }
}

pub(super) fn cfg() -> Cfg {
    cfg_at(crate::handshake::MINOR_VER)
}

/// The bytes a reply puts on the wire.
pub(super) fn encode_with<R>(resp: R, id: u64, cfg: Cfg) -> Vec<u8>
where
    R: EncodeResp,
    R::Error: std::fmt::Debug,
    crate::Error: From<R::Error>,
{
    let buf = resp.encode(id, cfg).expect("encodes").into_io_buf();

    buf.iter_slice().flatten().copied().collect()
}

pub(super) fn encode<R>(resp: R, id: u64) -> Vec<u8>
where
    R: EncodeResp,
    R::Error: std::fmt::Debug,
    crate::Error: From<R::Error>,
{
    encode_with(resp, id, cfg())
}

pub(super) fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

pub(super) fn i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

pub(super) fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

/// Checks what every successful reply has in common.
///
/// The kernel refuses a reply whose `len` disagrees with how many bytes
/// actually arrived (`oh.len != nbytes` in `fuse_dev_do_write`), so the header
/// has to count itself and the body exactly.
pub(super) fn assert_reply_header(bytes: &[u8], id: u64) {
    assert_eq!(
        u32_at(bytes, 0) as usize,
        bytes.len(),
        "the header's len does not count the whole reply"
    );
    assert_eq!(i32_at(bytes, 4), 0, "a successful reply carries no error");
    assert_eq!(u64_at(bytes, 8), id, "the reply answers the wrong request");
}
