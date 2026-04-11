use super::{Body, Ino, Mode, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{MkdirIn, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct MakeDirReq<'a> {
    req: Request,
    parent: Ino,
    mode: Mode,
    umask: Mode,
    name: &'a OsStr,
}

impl<'a> MakeDirReq<'a> {
    pub fn parent_ino(&self) -> Ino {
        self.parent
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
}

impl std::ops::Deref for MakeDirReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn mkdir<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, mut name) = decode::<MkdirIn>(body)?;
        let name_len = memchr::memchr(0, &name).unwrap_or(name.len());
        name.truncate(name_len);

        let mode = Mode::from_bits_retain(hdr.mode);
        let umask = Mode::from_bits_retain(hdr.umask);

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let minor = self.ver.1;

        spawn(async move {
            let ureq = MakeDirReq {
                req,
                parent: ino,
                mode,
                umask,
                name: OsStr::from_bytes(&name),
            };

            handle_error(match fs.make_dir(ureq).await {
                Ok(resp) => {
                    let body = resp.build();
                    if minor < 9 {
                        tx.send(MsgOut::new(req.id(), body.compat())).await
                    } else {
                        tx.send(MsgOut::new(req.id(), body)).await
                    }
                }
                Err(err) => send_error(err, req.id(), &mut tx).await,
            })
        })
        .detach();

        Ok(())
    }
}
