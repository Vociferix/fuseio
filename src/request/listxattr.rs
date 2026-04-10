use super::{Body, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{GetXattrIn, GetXattrOut, MsgOut};
use crate::serve::Server;
use crate::{BufPool, Error, Filesystem, Result};

use compio::buf::IoVectoredBuf;
use compio::runtime::spawn;
use futures_util::StreamExt;

#[derive(Debug)]
pub struct GetXattrKeysReq {
    req: Request,
    ino: Ino,
    max_len: usize,
    buf_pool: BufPool,
}

impl GetXattrKeysReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn max_len(&self) -> usize {
        self.max_len
    }

    pub fn buf_pool(&self) -> &BufPool {
        &self.buf_pool
    }
}

impl std::ops::Deref for GetXattrKeysReq {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn listxattr<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
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
                let ureq = GetXattrKeysReq {
                    req,
                    ino,
                    max_len: usize::MAX,
                    buf_pool: pool,
                };

                handle_error(match fs.get_xattr_keys_len(ureq).await {
                    Ok(stream) => {
                        let mut stream = std::pin::pin!(stream);

                        let mut total_len = 0usize;
                        while let Some(res) = stream.next().await {
                            match res {
                                Ok(len) => match total_len
                                    .checked_add(len)
                                    .and_then(|len| len.checked_add(1))
                                {
                                    Some(new_len) if new_len > max_len => {
                                        handle_error(
                                            send_error(Error::ERANGE, req.id(), &mut tx).await,
                                        );
                                        return;
                                    }
                                    Some(new_len) => total_len = new_len,
                                    None => {
                                        handle_error(
                                            send_error(Error::ERANGE, req.id(), &mut tx).await,
                                        );
                                        return;
                                    }
                                },
                                Err(err) => handle_error(send_error(err, req.id(), &mut tx).await),
                            }
                        }

                        let Ok(size) = u32::try_from(total_len) else {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        };
                        tx.send(MsgOut::new(req.id(), GetXattrOut { size, padding: 0 }))
                            .await
                    }
                    Err(err) => send_error(err, req.id(), &mut tx).await,
                });
            } else {
                let mut buf = pool.checkout_with_capacity::<u8>(max_len);
                let ureq = GetXattrKeysReq {
                    req,
                    ino,
                    max_len,
                    buf_pool: pool,
                };

                handle_error(match fs.get_xattr_keys(ureq).await {
                    Ok(stream) => {
                        let mut stream = std::pin::pin!(stream);
                        while let Some(res) = stream.next().await {
                            match res.map(crate::IntoIoBuf::into_io_buf) {
                                Ok(key) if buf.len() + key.total_len() >= max_len => {
                                    handle_error(
                                        send_error(Error::ERANGE, req.id(), &mut tx).await,
                                    );
                                    return;
                                }
                                Ok(key) => {
                                    let key = key.into_vectored();
                                    for bytes in key.iter_slice() {
                                        buf.extend_from_slice(bytes);
                                    }
                                    buf.push(0);
                                }
                                Err(err) => {
                                    handle_error(send_error(err, req.id(), &mut tx).await);
                                    return;
                                }
                            }
                        }

                        if buf.len() > max_len {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        }

                        let Ok(size) = u32::try_from(buf.len()) else {
                            handle_error(send_error(Error::ERANGE, req.id(), &mut tx).await);
                            return;
                        };

                        let mut hdr = MsgOut::new(req.id(), GetXattrOut { size, padding: 0 });
                        hdr.hdr.len += size;

                        tx.vsend((hdr, [buf])).await
                    }
                    Err(err) => send_error(err, req.id(), &mut tx).await,
                })
            }
        })
        .detach();

        Ok(())
    }
}
