use super::{Body, Ino, Request, decode};
use crate::async_rc::AsyncRc;
use crate::layout::{BatchForgetIn, ForgetOne};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct ForgetReq<'a> {
    pub(super) req: Request,
    pub(super) inodes: &'a [ForgetIno],
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ForgetIno {
    pub(super) inner: ForgetOne,
}

impl<'a> ForgetReq<'a> {
    pub fn inos(&self) -> &'a [ForgetIno] {
        self.inodes
    }
}

impl std::ops::Deref for ForgetReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl<'a> IntoIterator for ForgetReq<'a> {
    type Item = ForgetIno;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, ForgetIno>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inodes.iter().copied()
    }
}

impl<'a> IntoIterator for &ForgetReq<'a> {
    type Item = &'a ForgetIno;
    type IntoIter = std::slice::Iter<'a, ForgetIno>;

    fn into_iter(self) -> Self::IntoIter {
        self.inodes.iter()
    }
}

impl ForgetIno {
    pub fn ino(&self) -> Ino {
        unsafe { Ino::from_raw_unchecked(self.inner.nodeid) }
    }

    pub fn num_lookups(&self) -> u64 {
        self.inner.nlookup
    }
}

impl From<ForgetIno> for Ino {
    fn from(forget: ForgetIno) -> Ino {
        forget.ino()
    }
}

impl Server {
    pub fn batch_forget<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        _ino: Ino,
        body: Body,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let (subhdr, data) = decode::<BatchForgetIn>(body)?;

        let count = subhdr.count as usize;

        if count == 0 {
            return Ok(());
        }

        if data.len() < count * std::mem::size_of::<ForgetOne>() {
            return Err(Error::EINVAL);
        }

        let data: &[ForgetOne] =
            bytemuck::cast_slice(&data[..(count * std::mem::size_of::<ForgetOne>())]);
        for elem in data {
            if elem.nodeid == 0 {
                return Err(Error::EINVAL);
            }
        }

        let mut inos = self.bufs.checkout_with_capacity::<ForgetIno>(count);
        inos.extend_from_slice(bytemuck::cast_slice(
            &data[..(count * std::mem::size_of::<ForgetIno>())],
        ));

        let fs = fs.clone();

        spawn(async move {
            let ureq = ForgetReq { req, inodes: &inos };
            fs.forget(ureq).await;
        })
        .detach();

        Ok(())
    }
}
