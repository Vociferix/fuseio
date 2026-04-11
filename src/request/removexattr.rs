use super::{Body, Ino, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct RemoveXattrReq<'a> {
    req: Request,
    ino: Ino,
    key: &'a OsStr,
}

impl<'a> RemoveXattrReq<'a> {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &'a OsStr {
        self.key
    }
}

impl std::ops::Deref for RemoveXattrReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn removexattr<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        mut body: Body,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let key_len = memchr::memchr(0, &body).unwrap_or(body.len());
        body.truncate(key_len);

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = RemoveXattrReq {
                req,
                ino,
                key: OsStr::from_bytes(&body),
            };
            handle_error(send_result(fs.remove_xattr(ureq).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
