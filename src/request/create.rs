use super::{
    Body, Entry, FileHandle, Ino, InodeAttrs, Mode, OFlag, OpenAccessMode, OpenFlags, Opened,
    Request, decode, handle_error, send_error,
};
use crate::async_rc::AsyncRc;
use crate::channel::Sender;
use crate::layout::{CreateIn, CreateOut, CreateOutCompat, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, PassthroughFd, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

#[derive(Debug)]
pub struct CreateFileReq<'a> {
    pub(crate) req: Request,
    pub(crate) parent: Ino,
    pub(crate) flags: OFlag,
    pub(crate) mode: Mode,
    pub(crate) umask: Mode,
    pub(crate) name: &'a OsStr,
    pub(crate) dev: Sender,
}

#[derive(Debug)]
pub struct CreatedFile {
    open: Opened,
    entry: Entry,
}

impl CreatedFile {
    pub fn from_parts(entry: Entry, open: Opened) -> Self {
        Self { open, entry }
    }

    pub fn new(ino: Ino, fh: FileHandle) -> Self {
        Self {
            open: Opened::new(fh),
            entry: Entry::new(ino),
        }
    }

    pub fn flags(self, flags: OpenFlags) -> Self {
        let Self { open, entry } = self;
        Self {
            open: open.flags(flags),
            entry,
        }
    }

    pub fn add_flags(self, flags: OpenFlags) -> Self {
        let Self { open, entry } = self;
        Self {
            open: open.add_flags(flags),
            entry,
        }
    }

    pub fn passthrough<T: AsFd>(self, fd: &PassthroughFd<T>) -> Self {
        let Self { open, entry } = self;
        Self {
            open: open.passthrough(fd),
            entry,
        }
    }

    pub fn generation(self, generation: u64) -> Self {
        let Self { open, entry } = self;
        Self {
            open,
            entry: entry.generation(generation),
        }
    }

    pub fn ttl(self, ttl: Duration) -> Self {
        let Self { open, entry } = self;
        Self {
            open,
            entry: entry.ttl(ttl),
        }
    }

    pub fn attrs(self, attrs: InodeAttrs) -> Self {
        let Self { open, entry } = self;
        Self {
            open,
            entry: entry.attrs(attrs),
        }
    }
}

impl<'a> CreateFileReq<'a> {
    pub fn parent_ino(&self) -> Ino {
        self.parent
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

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn name(&self) -> &'a OsStr {
        self.name
    }

    pub fn open_passthrough<T: AsFd>(&self, fd: T) -> Result<PassthroughFd<T>> {
        PassthroughFd::open(fd, self.dev.clone())
    }

    pub fn as_make_inode_req(&self) -> super::MakeFileReq<'a> {
        super::MakeFileReq {
            req: self.req,
            parent: self.parent,
            mode: self.mode,
            umask: self.umask,
            rdev: 0,
            name: self.name,
        }
    }

    pub fn as_open_req(&self, entry: &Entry) -> super::OpenReq {
        super::OpenReq {
            req: self.req,
            ino: entry.ino,
            flags: self.flags,
            dev: self.dev.clone(),
        }
    }
}

impl std::ops::Deref for CreateFileReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn create<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, mut name) = decode::<CreateIn>(body)?;

        let name_len = memchr::memchr(0, &name).unwrap_or(name.len());
        name.truncate(name_len);

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let minor = self.ver.1;

        spawn(async move {
            let ureq = CreateFileReq {
                req,
                parent: ino,
                flags: OFlag::from_bits_retain(hdr.flags.cast_signed()),
                mode: Mode::from_bits_retain(hdr.mode),
                umask: Mode::from_bits_retain(hdr.mode),
                name: OsStr::from_bytes(&name),
                dev: tx.clone(),
            };
            handle_error(match fs.create_file(ureq).await {
                Ok(resp) => {
                    if minor < 9 {
                        tx.send(MsgOut::new(
                            req.id(),
                            CreateOutCompat {
                                entry: resp.entry.build().compat(),
                                open: resp.open.build(),
                            },
                        ))
                        .await
                    } else {
                        tx.send(MsgOut::new(
                            req.id(),
                            CreateOut {
                                entry: resp.entry.build(),
                                open: resp.open.build(),
                            },
                        ))
                        .await
                    }
                }
                Err(err) => send_error(err, req.id(), &mut tx).await,
            })
        })
        .detach();

        Ok(())
    }
}
