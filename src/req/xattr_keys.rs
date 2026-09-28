use super::Req;
use crate::buf::Buf;
use crate::proto::{request::ListXattr, response::Data};
use crate::types::Ino;
use crate::{Error, Result};

use futures_util::{Stream, StreamExt};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct XattrKeysReq<C> {
    req: Req<C>,
    listxattr: ListXattr,
}

#[derive(Debug)]
pub struct XattrKeyBuf {
    pub(crate) max_len: usize,
    pub(crate) buf: Buf,
}

impl<C> XattrKeysReq<C> {
    pub(crate) fn new(req: Req<C>, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }

    // Not a collection: this is a byte count the kernel asked for, and a zero
    // one is meaningful rather than "empty".
    #[allow(clippy::len_without_is_empty)]
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

    pub fn try_collect_keys<I, T, E>(&self, iter: I) -> Result<XattrKeyBuf>
    where
        I: IntoIterator<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        let mut buf = self.new_buffer();
        buf.try_extend(iter)?;
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

    pub async fn try_collect_keys_stream<S, T, E>(&self, stream: S) -> Result<XattrKeyBuf>
    where
        S: Stream<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        let mut buf = self.new_buffer();
        buf.try_extend_stream(stream).await?;
        Ok(buf)
    }
}

impl<C> std::ops::Deref for XattrKeysReq<C> {
    type Target = Req<C>;

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

        // listxattr(2) returns a sequence of NUL-terminated names.
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

        self.buf.extend_from_slice(key);
        self.buf.push(0);

        Ok(())
    }

    pub fn extend<I>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        iter.into_iter().try_for_each(|key| self.push(key))
    }

    pub fn try_extend<I, T, E>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        iter.into_iter().try_for_each(|key| {
            self.push(match key {
                Ok(key) => key,
                Err(err) => return Err(err.into()),
            })
        })
    }

    pub async fn extend_stream<S>(&mut self, stream: S) -> Result<()>
    where
        S: Stream,
        S::Item: AsRef<OsStr>,
    {
        let mut stream = std::pin::pin!(stream);

        while let Some(key) = stream.next().await {
            self.push(key)?;
        }

        Ok(())
    }

    pub async fn try_extend_stream<S, T, E>(&mut self, stream: S) -> Result<()>
    where
        S: Stream<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        let mut stream = std::pin::pin!(stream);

        while let Some(key) = stream.next().await {
            self.push(match key {
                Ok(key) => key,
                Err(err) => return Err(err.into()),
            })?;
        }

        Ok(())
    }

    pub fn with_keys<I>(mut self, iter: I) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        self.extend(iter)?;
        Ok(self)
    }

    pub fn try_with_keys<I, T, E>(mut self, iter: I) -> Result<Self>
    where
        I: IntoIterator<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        self.try_extend(iter)?;
        Ok(self)
    }

    pub async fn with_keys_stream<S>(mut self, stream: S) -> Result<Self>
    where
        S: Stream,
        S::Item: AsRef<OsStr>,
    {
        self.extend_stream(stream).await?;
        Ok(self)
    }

    pub async fn try_with_keys_stream<S, T, E>(mut self, stream: S) -> Result<Self>
    where
        S: Stream<Item = std::result::Result<T, E>>,
        T: AsRef<OsStr>,
        E: Into<Error>,
    {
        self.try_extend_stream(stream).await?;
        Ok(self)
    }

    pub(crate) fn into_data(self) -> Data<Buf> {
        Data::new(self.buf)
    }
}
