use super::{Body, FileHandle, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{CopyFileRangeIn, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct CopyFileRangeReq {
    req: Request,
    src: CopyFileRangePos,
    dst: CopyFileRangePos,
    len: u64,
}

#[derive(Debug)]
pub struct CopyFileRangePos {
    ino: Ino,
    fh: FileHandle,
    off: u64,
}

impl CopyFileRangeReq {
    pub fn src(&self) -> &CopyFileRangePos {
        &self.src
    }

    pub fn dst(&self) -> &CopyFileRangePos {
        &self.dst
    }

    pub fn num_bytes(&self) -> u64 {
        self.len
    }
}

impl std::ops::Deref for CopyFileRangeReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl CopyFileRangePos {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.off
    }
}

impl Server {
    pub fn copy_file_range<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        body: Body,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<CopyFileRangeIn>(body)?.0;

        let ino_out = Ino::from_raw(body.nodeid_out).ok_or(Error::EINVAL)?;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = CopyFileRangeReq {
                req,
                src: CopyFileRangePos {
                    ino,
                    fh: FileHandle(body.fh_in),
                    off: body.off_in,
                },
                dst: CopyFileRangePos {
                    ino: ino_out,
                    fh: FileHandle(body.fh_out),
                    off: body.off_out,
                },
                len: body.len,
            };

            handle_error(match fs.copy_file_range(ureq).await {
                Ok(count) => tx.send(MsgOut::new(req.id(), count)).await,
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
