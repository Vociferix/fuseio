use super::{Body, FileHandle, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{LseekIn, LseekOut, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;
use nix::unistd::Whence as NixWhence;

#[derive(Debug)]
pub struct LseekReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    whence: Whence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Whence {
    Data,
    Hole,
}

impl LseekReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn whence(&self) -> Whence {
        self.whence
    }
}

impl std::ops::Deref for LseekReq {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl TryFrom<NixWhence> for Whence {
    type Error = std::io::Error;

    fn try_from(whence: NixWhence) -> std::result::Result<Self, Self::Error> {
        match whence {
            NixWhence::SeekData => Ok(Self::Data),
            NixWhence::SeekHole => Ok(Self::Hole),
            _ => Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
}

impl From<Whence> for NixWhence {
    fn from(whence: Whence) -> Self {
        match whence {
            Whence::Data => Self::SeekData,
            Whence::Hole => Self::SeekHole,
        }
    }
}

impl Server {
    pub fn lseek<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        const SEEK_DATA: u32 = nix::libc::SEEK_DATA as u32;
        const SEEK_HOLE: u32 = nix::libc::SEEK_HOLE as u32;

        let hdr = decode::<LseekIn>(body)?.0;
        let whence = match hdr.whence {
            SEEK_DATA => Whence::Data,
            SEEK_HOLE => Whence::Hole,
            _ => return Err(Error::EINVAL),
        };
        let offset = u64::try_from(hdr.offset).unwrap_or(0);
        let fh = FileHandle(hdr.fh);

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = LseekReq {
                req,
                ino,
                fh,
                offset,
                whence,
            };

            handle_error(match fs.lseek(ureq).await {
                Ok(offset) => {
                    tx.send(MsgOut::new(
                        req.id(),
                        LseekOut {
                            offset: offset as i64,
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
