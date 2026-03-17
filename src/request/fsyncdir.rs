use super::{Body, FileHandle, Ino, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::{FsyncFlags, FsyncIn};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

pub struct FsyncDirReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    datasync: bool,
}

impl FsyncDirReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn datasync(&self) -> bool {
        self.datasync
    }
}

impl std::ops::Deref for FsyncDirReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn fsyncdir<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<FsyncIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = FsyncDirReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                datasync: body.fsync_flags.contains(FsyncFlags::DATASYNC),
            };

            handle_error(send_result(fs.fsyncdir(&req).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
