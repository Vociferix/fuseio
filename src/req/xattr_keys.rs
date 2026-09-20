use super::Req;
use crate::buf::{Buf, BufPool};
use crate::proto::{request::ListXattr, response::Data};
use crate::types::Ino;
use crate::{Error, Result};

use futures_util::{Stream, StreamExt};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct XattrKeysReq {
    req: Req,
    listxattr: ListXattr,
}

#[derive(Debug)]
pub struct XattrKeyBuf {
    pub(crate) max_len: usize,
    pub(crate) buf: Buf,
}

impl XattrKeysReq {
    pub(crate) fn new(req: Req, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }

    pub fn len(&self) -> usize {
        self.listxattr.len()
    }

    pub fn new_buffer(&self) -> XattrKeyBuf {
        let len = self.len();
        XattrKeyBuf {
            max_len: len,
            buf: self.buffer_pool().checkout_with_capacity(len),
        }
    }

    pub fn collect_keys<I>(&self, iter: I) -> Result<XattrKeyBuf>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend(iter)?;
        Ok(buf)
    }

    pub async fn collect_keys_stream<S>(&self, stream: S) -> Result<XattrKeyBuf>
    where
        S: Stream,
        S::Item: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend_stream(stream).await?;
        Ok(buf)
    }
}

impl std::ops::Deref for XattrKeysReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl XattrKeyBuf {
    pub fn push<K>(&mut self, key: K) -> Result<()>
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref().as_bytes();
        if self.buf.is_empty() {
            let Some(new_len) = self.buf.len().checked_add(key.len()) else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.extend_from_slice(key);
        } else {
            let Some(new_len) = self
                .buf
                .len()
                .checked_add(key.len())
                .and_then(|len| len.checked_add(1))
            else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.push(0);
            self.buf.extend_from_slice(key);
        }
        Ok(())
    }

    pub fn extend<I>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        let mut iter = iter.into_iter();
        if self.buf.is_empty() {
            let Some(key) = iter.next() else {
                return Ok(());
            };
            let key = key.as_ref().as_bytes();

            let Some(new_len) = self.buf.len().checked_add(key.len()) else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.extend_from_slice(key);
        }

        for key in iter {
            let key = key.as_ref().as_bytes();
            let Some(new_len) = self
                .buf
                .len()
                .checked_add(key.len())
                .and_then(|len| len.checked_add(1))
            else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.push(0);
            self.buf.extend_from_slice(key);
        }

        Ok(())
    }

    pub async fn extend_stream<S>(&mut self, stream: S) -> Result<()>
    where
        S: Stream,
        S::Item: AsRef<OsStr>,
    {
        let mut stream = std::pin::pin!(stream);

        if self.buf.is_empty() {
            let Some(key) = stream.next().await else {
                return Ok(());
            };
            let key = key.as_ref().as_bytes();

            let Some(new_len) = self.buf.len().checked_add(key.len()) else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.extend_from_slice(key);
        }

        while let Some(key) = stream.next().await {
            let key = key.as_ref().as_bytes();
            let Some(new_len) = self
                .buf
                .len()
                .checked_add(key.len())
                .and_then(|len| len.checked_add(1))
            else {
                return Err(Error::ERANGE);
            };
            if new_len > self.max_len {
                return Err(Error::ERANGE);
            }
            self.buf.push(0);
            self.buf.extend_from_slice(key);
        }

        Ok(())
    }

    pub(crate) fn into_data(self) -> Data<Buf> {
        Data::new(self.buf)
    }
}
