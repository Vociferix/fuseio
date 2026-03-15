use super::{FileHandle, Ino, OFlag, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::channel::Sender;
use crate::layout::{MsgOut, OpenFlags as RawOpenFlags, OpenIn, OpenOut};
use crate::serve::Server;
use crate::{BackingId, Error, Filesystem, PassthroughFd, Result};

use compio::runtime::spawn;

use std::os::fd::AsFd;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct OpenFlags: u32 {
        const DIRECT_IO = 1 << 0;
        const KEEP_CACHE = 1 << 1;
        const NONSEEKABLE = 1 << 2;
        const CACHE_DIR = 1 << 3;
        const STREAM = 1 << 4;
        const NO_FLUSH = 1 << 5;
        const PARALLEL_DIRECT_WRITES = 1 << 6;
    }
}

#[derive(Debug)]
pub struct OpenReq {
    req: Request,
    ino: Ino,
    flags: OFlag,
    dev: Sender,
}

#[derive(Debug)]
pub struct OpenResp {
    fh: FileHandle,
    flags: OpenFlags,
    backing_id: Option<BackingId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpenAccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl OpenReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

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

    pub fn open_passthrough<T: AsFd>(&self, fd: T) -> Result<PassthroughFd<T>> {
        PassthroughFd::open(fd, self.dev.clone())
    }
}

impl std::ops::Deref for OpenReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl OpenAccessMode {
    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }

    pub const fn is_write_only(self) -> bool {
        matches!(self, Self::WriteOnly)
    }

    pub const fn is_read_write(self) -> bool {
        matches!(self, Self::ReadWrite)
    }

    pub const fn wants_read(self) -> bool {
        !self.is_write_only()
    }

    pub const fn wants_write(self) -> bool {
        !self.is_read_only()
    }
}

impl OpenResp {
    pub fn new(fh: FileHandle) -> Self {
        Self {
            fh,
            flags: OpenFlags::empty(),
            backing_id: None,
        }
    }

    pub fn flags(mut self, flags: OpenFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn add_flags(mut self, flags: OpenFlags) -> Self {
        self.flags |= flags;
        self
    }

    pub fn passthrough<T: AsFd>(mut self, fd: &PassthroughFd<T>) -> Self {
        self.backing_id = Some(PassthroughFd::backing_id(fd));
        self
    }
}

impl Server {
    pub fn open<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<OpenIn>(body)?.0;

        let flags = OFlag::from_bits_retain(body.flags.cast_signed());
        if !matches!(
            flags & OFlag::O_ACCMODE,
            OFlag::O_RDONLY | OFlag::O_WRONLY | OFlag::O_RDWR
        ) {
            return Err(Error::EINVAL);
        }

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = OpenReq {
                req,
                ino,
                flags,
                dev: tx.clone(),
            };
            handle_error(match fs.open(&req).await {
                Ok(resp) => {
                    let flags =
                        RawOpenFlags::from_bits_retain((resp.flags & OpenFlags::all()).bits());
                    let (flags, backing_id) = if let Some(backing_id) = resp.backing_id {
                        (flags | RawOpenFlags::PASSTHROUGH, backing_id.as_raw())
                    } else {
                        (flags, 0)
                    };
                    tx.send(MsgOut::new(
                        req.id(),
                        OpenOut {
                            fh: resp.fh.into(),
                            open_flags: flags,
                            backing_id,
                        },
                    ))
                    .await
                }
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
