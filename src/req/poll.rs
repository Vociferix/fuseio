use super::Req;
use crate::context::Context;
use crate::dev_fuse::FuseChannel;
use crate::proto::{Cfg, notify::EncodeNotify, request::Poll};
use crate::types::{FileHandle, Ino, PollFlags};

use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug)]
pub struct PollReq {
    req: Req,
    poll: Poll,
    handle_taken: Cell<bool>,
}

#[derive(Debug)]
pub struct PollNotify {
    id: u64,
    ctx: Context,
}

impl PollReq {
    pub(crate) fn new(req: Req, poll: Poll) -> Self {
        Self {
            req,
            poll,
            handle_taken: Cell::new(false),
        }
    }

    pub fn req(&self) -> &Req {
        &self.req
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

impl std::ops::Deref for PollReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl PollNotify {
    pub(crate) fn new(req: &PollReq, id: u64) -> Self {
        Self {
            id,
            ctx: req.ctx.clone(),
        }
    }

    pub async fn notify(self) -> crate::Result<()> {
        let Self { id, ctx } = self;

        ctx.dev()
            .write_buf(crate::proto::notify::Poll::new(id).encode(ctx.cfg())?)
            .await
            .0?;

        Ok(())
    }
}
