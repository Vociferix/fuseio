use super::NotifyCaps;

bitflags::bitflags! {
    /// What a connection can carry out, as opposed to what the protocol version
    /// or the INIT negotiation allows.
    ///
    /// A peer may advertise a protocol version whose features it never
    /// implemented, and an operation may need a side channel the transport
    /// doesn't have. These answer for the connection itself.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct ConnCaps : u64 {
        const PASSTHROUGH = 1 << 0;
        const INDEPENDENT_CLONES = 1 << 1;
        const ABORT = 1 << 2;

        const SYNCFS = 1 << 3;
        const RENAME2 = 1 << 4;
        const POLL = 1 << 5;
        const MONITOR = 1 << 6;

        const MAX_PAGES = 1 << 7;

        /// The peer asks for a volume's name to be set.
        const VOLUME_NAME = 1 << 8;

        /// The peer asks for the backup and creation times of an inode.
        const BACKUP_TIMES = 1 << 9;

        /// The peer asks for two files to be exchanged.
        const EXCHANGE_DATA = 1 << 10;
    }
}

/// What a connection advertises about the other end, as a value rather than a
/// borrow.
///
/// [`KernelConfig`](crate::fs::types::KernelConfig) outlives no connection and
/// so cannot hold one, and tests need to ask a feature question about a peer
/// that isn't the one this process is talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeerCaps {
    pub(crate) caps: ConnCaps,
    pub(crate) notify: NotifyCaps,
}

impl PeerCaps {
    pub(crate) const fn new(caps: ConnCaps, notify: NotifyCaps) -> Self {
        Self { caps, notify }
    }

    pub(crate) fn of<C>(conn: &C) -> Self
    where
        C: crate::conn::ConnectionMeta,
    {
        Self::new(conn.capabilities(), conn.notify_capabilities())
    }
}

#[cfg(test)]
impl PeerCaps {
    /// A peer that gates nothing, so a test can isolate the protocol version and
    /// the negotiated capabilities.
    pub(crate) const PERMISSIVE: Self = Self::new(ConnCaps::all(), NotifyCaps::all());
}
