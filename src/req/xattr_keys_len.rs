use super::Req;
use crate::buf::Buf;
use crate::proto::request::ListXattr;
use crate::types::Ino;
use crate::{Error, Result};

use futures_util::{Stream, StreamExt};

#[derive(Debug)]
pub struct XattrKeysLenReq {
    req: Req,
    listxattr: ListXattr,
}

#[derive(Debug)]
pub struct XattrKeyLenBuf {
    pub(crate) len: u32,
}

impl XattrKeysLenReq {
    pub(crate) fn new(req: Req, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }

    pub fn new_buffer(&self) -> XattrKeyLenBuf {
        XattrKeyLenBuf { len: 0 }
    }
}

impl std::ops::Deref for XattrKeysLenReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl XattrKeyLenBuf {
    pub fn push(&mut self, len: usize) -> Result<()> {
        let Ok(len) = u32::try_from(len) else {
            return Err(Error::E2BIG);
        };
        if let Some(len) = self.len.checked_add(len).and_then(|len| len.checked_add(1)) {
            self.len = len;
            Ok(())
        } else {
            Err(Error::E2BIG)
        }
    }

    pub fn extend<I>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = usize>,
    {
        iter.into_iter().try_for_each(|len| self.push(len))
    }

    pub fn try_extend<I, E>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = std::result::Result<usize, E>>,
        E: Into<Error>,
    {
        iter.into_iter().try_for_each(|len| {
            self.push(match len {
                Ok(len) => len,
                Err(err) => return Err(err.into()),
            })
        })
    }

    pub async fn extend_stream<S>(&mut self, stream: S) -> Result<()>
    where
        S: Stream<Item = usize>,
    {
        let mut stream = std::pin::pin!(stream);
        while let Some(len) = stream.next().await {
            self.push(len)?;
        }
        Ok(())
    }

    pub async fn try_extend_stream<S, E>(&mut self, stream: S) -> Result<()>
    where
        S: Stream<Item = std::result::Result<usize, E>>,
        E: Into<Error>,
    {
        let mut stream = std::pin::pin!(stream);
        while let Some(len) = stream.next().await {
            self.push(match len {
                Ok(len) => len,
                Err(err) => return Err(err.into()),
            })?;
        }
        Ok(())
    }

    pub fn with_lengths<I>(mut self, iter: I) -> Result<Self>
    where
        I: IntoIterator<Item = usize>,
    {
        self.extend(iter)?;
        Ok(self)
    }

    pub fn try_with_lengths<I, E>(mut self, iter: I) -> Result<Self>
    where
        I: IntoIterator<Item = std::result::Result<usize, E>>,
        E: Into<Error>,
    {
        self.try_extend(iter)?;
        Ok(self)
    }

    pub async fn with_lengths_stream<S>(mut self, stream: S) -> Result<Self>
    where
        S: Stream<Item = usize>,
    {
        self.extend_stream(stream).await?;
        Ok(self)
    }

    pub async fn try_with_lengths_stream<S, E>(mut self, stream: S) -> Result<Self>
    where
        S: Stream<Item = std::result::Result<usize, E>>,
        E: Into<Error>,
    {
        self.try_extend_stream(stream).await?;
        Ok(self)
    }
}
