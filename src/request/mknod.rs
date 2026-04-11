use super::{Body, Ino, Mode, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{MknodIn, MknodInCompat, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct MakeFileReq<'a> {
    pub(crate) req: Request,
    pub(crate) parent: Ino,
    pub(crate) mode: Mode,
    pub(crate) umask: Mode,
    pub(crate) rdev: u32,
    pub(crate) name: &'a OsStr,
}

impl<'a> MakeFileReq<'a> {
    pub fn parent_ino(&self) -> Ino {
        self.parent
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn device_num(&self) -> u32 {
        self.rdev
    }

    pub fn name(&self) -> &'a OsStr {
        self.name
    }
}

impl std::ops::Deref for MakeFileReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn mknod<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, mut name) = if self.ver.1 < 12 {
            let (hdr, name) = decode::<MknodInCompat>(body)?;
            (MknodIn::from_compat(hdr), name)
        } else {
            decode::<MknodIn>(body)?
        };

        let name_len = memchr::memchr(0, &name).unwrap_or(name.len());
        name.truncate(name_len);

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let minor = self.ver.1;

        spawn(async move {
            let ureq = MakeFileReq {
                req,
                parent: ino,
                mode: Mode::from_bits_retain(hdr.mode),
                umask: Mode::from_bits_retain(hdr.umask),
                rdev: hdr.rdev,
                name: OsStr::from_bytes(&name),
            };
            handle_error(match fs.make_file(ureq).await {
                Ok(resp) => {
                    let body = resp.build();
                    if minor < 9 {
                        tx.send(MsgOut::new(req.id(), body.compat())).await
                    } else {
                        tx.send(MsgOut::new(req.id(), body)).await
                    }
                }
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
