use super::Req;
use crate::dev_fuse::FuseChannel;
use crate::proto::{Cfg, notify::EncodeNotify, request::Poll};
use crate::types::{FileHandle, Ino, PollFlags};

use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug)]
pub struct PollReq<'a> {
    req: Req<'a>,
    poll: Poll,
    handle_taken: Cell<bool>,
}

#[derive(Debug)]
pub struct PollNotify {
    id: u64,
    dev: Rc<FuseChannel>,
    cfg: Cfg,
}

impl<'a> PollReq<'a> {
    pub(crate) fn new(req: Req<'a>, poll: Poll) -> Self {
        Self {
            req,
            poll,
            handle_taken: Cell::new(false),
        }
    }

    pub fn ino(&self) -> Ino {
        self.poll.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.poll.file_handle()
    }

    pub fn poll_handle(&self) -> Option<PollNotify> {
        if self.handle_taken.replace(true) {
            return None;
        }
        self.poll.poll_handle().map(|id| PollNotify::new(self, id))
    }

    pub fn interests(&self) -> PollFlags {
        self.poll.interests()
    }
}

impl<'a> std::ops::Deref for PollReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl PollNotify {
    pub(crate) fn new(req: &PollReq<'_>, id: u64) -> Self {
        Self {
            id,
            dev: req.req.server.dev.clone(),
            cfg: Cfg {
                minor_ver: req.req.server.minor_ver,
                flags: req.req.server.flags,
            },
        }
    }

    pub async fn notify(self) -> crate::Result<()> {
        let Self { id, dev, cfg } = self;

        dev.write_buf(crate::proto::notify::Poll::new(id).encode(cfg)?)
            .await
            .0?;

        Ok(())
    }
}
