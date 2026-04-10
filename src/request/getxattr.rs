use super::{Body, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{GetXattrIn, GetXattrOut, MsgOut};
use crate::serve::Server;
use crate::{BufPool, Error, Filesystem, IntoIoBuf, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct GetXattrReq<'a> {
    req: Request,
    ino: Ino,
    key: &'a OsStr,
    max_len: usize,
    buf_pool: BufPool,
}

impl<'a> GetXattrReq<'a> {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &'a OsStr {
        self.key
    }

    pub fn max_len(&self) -> usize {
        self.max_len
    }

    pub fn buf_pool(&self) -> &BufPool {
        &self.buf_pool
    }
}

impl std::ops::Deref for GetXattrReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn getxattr<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, mut name) = decode::<GetXattrIn>(body)?;

        #[cfg(target_os = "macos")]
        {
            if hdr.pos != 0 {
                return Err(Error::EINVAL);
            }
        }

        let name_len = memchr::memchr(0, &name).unwrap_or(name.len());
        name.truncate(name_len);

        let max_len = hdr.size as usize;

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let pool = self.bufs.clone();

        spawn(async move {
            if max_len == 0 {
                let ureq = GetXattrReq {
                    req,
                    ino,
                    key: OsStr::from_bytes(&name),
                    max_len: usize::MAX,
                    buf_pool: pool,
                };

                handle_error(match fs.get_xattr_len(ureq).await {
                    Ok(len) => {
                        let Ok(size) = u32::try_from(len) else {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        };
                        tx.send(MsgOut::new(req.id(), GetXattrOut { size, padding: 0 }))
                            .await
                    }
                    Err(err) => send_error(err, req.id(), &mut tx).await,
                })
            } else {
                let ureq = GetXattrReq {
                    req,
                    ino,
                    key: OsStr::from_bytes(&name),
                    max_len,
                    buf_pool: pool,
                };

                handle_error(match fs.get_xattr(ureq).await {
                    Ok(buf) => {
                        let buf = buf.into_io_buf();
                        let len = buf.total_len();
                        if len > max_len {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        }
                        let Ok(size) = u32::try_from(len) else {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        };

                        let mut hdr = MsgOut::new(req.id(), GetXattrOut { size, padding: 0 });
                        hdr.hdr.len += size;

                        tx.vsend((hdr, buf.into_vectored())).await
                    }
                    Err(err) => send_error(err, req.id(), &mut tx).await,
                });
            }
        })
        .detach();

        Ok(())
    }
}
