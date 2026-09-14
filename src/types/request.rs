use super::{Gid, Pid, Uid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request {
    pub(crate) id: u64,
    pub(crate) uid: Uid,
    pub(crate) gid: Gid,
    pub(crate) pid: Pid,
}

impl Request {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn uid(&self) -> Uid {
        self.uid
    }

    pub fn gid(&self) -> Gid {
        self.gid
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }
}
