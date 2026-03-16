use super::{FileHandle, Ino, LockOwner, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::FlushIn;
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct FlushReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    lock_owner: LockOwner,
}

impl FlushReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> LockOwner {
        self.lock_owner
    }
}

impl std::ops::Deref for FlushReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn flush<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<FlushIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = FlushReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                lock_owner: LockOwner(body.lock_owner),
            };
            handle_error(send_result(fs.flush(&req).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
