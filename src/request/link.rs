use super::{Body, Entry, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{LinkIn, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct LinkReq<'a> {
    req: Request,
    new_parent: Ino,
    src: Ino,
    name: &'a OsStr,
}

impl<'a> LinkReq<'a> {
    pub fn parent(&self) -> Ino {
        self.new_parent
    }

    pub fn source(&self) -> Ino {
        self.src
    }

    pub fn name(&self) -> &'a OsStr {
        self.name
    }
}

impl std::ops::Deref for LinkReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn link<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, mut name) = decode::<LinkIn>(body)?;

        let Some(src) = Ino::from_raw(hdr.oldnodeid) else {
            return Err(Error::EINVAL);
        };

        let name_len = memchr::memchr(0, &name).unwrap_or(name.len());
        name.truncate(name_len);

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = LinkReq {
                req,
                new_parent: ino,
                src,
                name: OsStr::from_bytes(&name),
            };
            handle_error(match fs.link(ureq).await {
                Ok(resp) => tx.send(MsgOut::new(req.id(), resp.build())).await,
                Err(err) => send_error(err, req.id(), &mut tx).await,
            })
        })
        .detach();

        Ok(())
    }
}
