//! Scaffolding for notification-encoder tests.
//!
//! A notification is a reply with no request behind it: the kernel recognises
//! one by a zero `unique`, and reads the notification code out of the field a
//! reply would use for its error (`fuse_dev_do_write`). That branch is taken
//! before the check rejecting a positive error, which is why the code travels
//! positive.

use super::{Cfg, EncodeNotify};
use crate::buf::IntoIoBuf;

use compio::buf::IoVectoredBuf;

/// `struct fuse_out_header`: `len`, `error`, `unique`.
pub(super) const HDR: usize = 16;

pub(super) fn cfg() -> Cfg {
    Cfg {
        minor_ver: crate::handshake::MINOR_VER,
        ..Cfg::default()
    }
}

/// The bytes a notification puts on the wire.
pub(super) fn encode<N>(notify: N) -> Vec<u8>
where
    N: EncodeNotify,
    N::Error: std::fmt::Debug,
    crate::Error: From<N::Error>,
{
    let buf = notify.encode(cfg()).expect("encodes").into_io_buf();

    buf.iter_slice().flatten().copied().collect()
}

pub(super) fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

pub(super) fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

/// Checks what every notification has in common: the right code, a zero
/// `unique`, and a `len` counting the whole message.
pub(super) fn assert_notify_header(bytes: &[u8], code: u32) {
    assert_eq!(
        u32_at(bytes, 0) as usize,
        bytes.len(),
        "the header's len does not count the whole notification"
    );
    assert_eq!(u32_at(bytes, 4), code, "the wrong notification code");
    assert_eq!(
        u64_at(bytes, 8),
        0,
        "a notification must have a zero unique"
    );
}
