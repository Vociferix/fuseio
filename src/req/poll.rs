use super::Req;
use crate::conn::Connection;
use crate::context::Context;
use crate::proto::{notify::EncodeNotify, request::Poll};
use crate::types::{FileHandle, Ino, PollFlags};

use std::cell::Cell;

#[derive(Debug)]
pub struct PollReq<C> {
    req: Req<C>,
    poll: Poll,
    handle_taken: Cell<bool>,
}

#[derive(Debug)]
pub struct PollNotify<C> {
    id: u64,
    ctx: Context<C>,
}

impl<C> PollReq<C> {
    pub(crate) fn new(req: Req<C>, poll: Poll) -> Self {
        Self {
            req,
            poll,
            handle_taken: Cell::new(false),
        }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.poll.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.poll.file_handle()
    }

    pub fn poll_handle(&self) -> Option<PollNotify<C>> {
        if self.handle_taken.replace(true) {
            return None;
        }
        self.poll.poll_handle().map(|id| PollNotify::new(self, id))
    }

    pub fn interests(&self) -> PollFlags {
        self.poll.interests()
    }
}

impl<C> std::ops::Deref for PollReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl<C> PollNotify<C> {
    pub(crate) fn new(req: &PollReq<C>, id: u64) -> Self {
        Self {
            id,
            ctx: req.ctx.clone(),
        }
    }
}

impl<C: Connection> PollNotify<C> {
    pub async fn notify(self) -> Result<(), crate::fs::types::NotifyError> {
        let Self { id, ctx } = self;

        ctx.send_notif(crate::proto::notify::Poll::new(id).encode(ctx.cfg())?)
            .await
    }
}
