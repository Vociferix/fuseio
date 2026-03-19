use super::{Body, FallocateFlags, FileHandle, Ino, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::FallocateIn;
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct FallocateReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: u64,
    mode: FallocateFlags,
}

impl FallocateReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn mode(&self) -> FallocateFlags {
        self.mode
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

impl std::ops::Deref for FallocateReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn fallocate<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<FallocateIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = FallocateReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                offset: body.offset,
                len: body.length,
                mode: FallocateFlags::from_bits_retain(body.mode.cast_signed()),
            };
            handle_error(send_result(fs.fallocate(ureq).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
