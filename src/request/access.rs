use super::{AccessFlags, Ino, Request, send_result};
use crate::Filesystem;
use crate::async_rc::AsyncRc;
use crate::layout::AccessIn;
use crate::serve::Server;

use compio::runtime::spawn;

use std::io::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessReq {
    req: Request,
    ino: Ino,
    flags: AccessFlags,
}

impl AccessReq {
    pub fn flags(&self) -> AccessFlags {
        self.flags
    }
}

impl std::ops::Deref for AccessReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn access<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let Some(body) = self.decode::<AccessIn>(body, fs, &req) else {
            return Ok(());
        };

        let mut tx = self.tx.clone();
        let fs = fs.clone();
        let req = AccessReq {
            req,
            ino,
            flags: AccessFlags::from_bits_retain(body.mask.cast_signed()),
        };

        spawn(async move {
            let res = fs.access(&req).await;
            let _ = send_result(res, req.id(), &mut tx).await;
        })
        .detach();

        Ok(())
    }
}
