//! Scaffolding for decoder tests: wire requests built by hand.
//!
//! Decoders read bytes the kernel wrote, so a test says what the kernel would
//! have sent and asserts what the decoder made of it. Nothing here goes through
//! another decoder, so a test cannot be fooled by the same mistake twice.

use super::{Cfg, HDR_LEN, Opcode};
use crate::buf::{Buf, BufPool};
use crate::types::{Ino, ReplyInitFlags};

/// The root inode, which `FUSE_ROOT_ID` fixes at one.
pub(super) const ROOT: u64 = 1;

/// A request header naming `nodeid`, followed by `body`.
///
/// The header's `len` counts itself, as the kernel's does.
pub(super) fn request_on(op: Opcode, nodeid: u64, body: &[u8]) -> Buf {
    let len = HDR_LEN + body.len();
    let mut buf = BufPool::new().checkout_with_capacity(len);

    buf.extend_from_slice(&(len as u32).to_ne_bytes());
    buf.extend_from_slice(&op.0.to_ne_bytes());
    buf.extend_from_slice(&7u64.to_ne_bytes()); // unique
    buf.extend_from_slice(&nodeid.to_ne_bytes());
    buf.extend_from_slice(&1000u32.to_ne_bytes()); // uid
    buf.extend_from_slice(&1000u32.to_ne_bytes()); // gid
    buf.extend_from_slice(&4242u32.to_ne_bytes()); // pid
    buf.extend_from_slice(&0u32.to_ne_bytes()); // total_extlen, padding

    assert_eq!(buf.len(), HDR_LEN, "the header is not the size it claims");

    buf.extend_from_slice(body);
    buf
}

/// As [`request_on`], on the root inode.
pub(super) fn request(op: Opcode, body: &[u8]) -> Buf {
    request_on(op, ROOT, body)
}

/// The nodeid [`request`] names, as the decoders receive it.
pub(super) fn root() -> Option<Ino> {
    Ino::from_raw(ROOT)
}

/// A peer speaking protocol 7.`minor`, with nothing negotiated.
pub(super) fn cfg(minor: u32) -> Cfg {
    Cfg {
        minor_ver: minor,
        flags: ReplyInitFlags::empty(),
    }
}

/// A peer speaking the newest version this crate does.
pub(super) fn latest() -> Cfg {
    cfg(crate::handshake::MINOR_VER)
}
