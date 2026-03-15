use super::{
    Ino, Mode, OFlag, OpenAccessMode, OpenFlags, Request, decode, handle_error, send_error,
};
use crate::async_rc::AsyncRc;
use crate::channel::Sender;
use crate::layout::{CreateIn, CreateOut, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, PassthroughFd, Result};

use compio::runtime::spawn;

use std::os::fd::AsFd;

#[derive(Debug)]
pub struct CreateReq {
    req: Request,
    flags: OFlag,
    mode: Mode,
    umask: Mode,
    dev: Sender,
}

impl CreateReq {
    pub fn flags(&self) -> OFlag {
        self.flags
    }

    pub fn access_mode(&self) -> OpenAccessMode {
        match self.flags & OFlag::O_ACCMODE {
            OFlag::O_RDONLY => OpenAccessMode::ReadOnly,
            OFlag::O_WRONLY => OpenAccessMode::WriteOnly,
            OFlag::O_RDWR => OpenAccessMode::ReadWrite,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn open_passthrough<T: AsFd>(&self, fd: T) -> Result<PassthroughFd<T>> {
        PassthroughFd::open(fd, self.dev.clone())
    }
}

impl std::ops::Deref for CreateReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn create<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<CreateIn>(body)?;

        todo!()
    }
}
