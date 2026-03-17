use super::{AccessFlags, Body, Ino, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::AccessIn;
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

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
    pub fn access<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<AccessIn>(body)?.0;

        let mut tx = self.tx.clone();
        let fs = fs.clone();
        let req = AccessReq {
            req,
            ino,
            flags: AccessFlags::from_bits_retain(body.mask.cast_signed()),
        };

        spawn(async move {
            let res = fs.access(&req).await;
            handle_error(send_result(res, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
