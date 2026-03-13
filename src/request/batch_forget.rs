use super::{Ino, Request, send_error};
use crate::Filesystem;
use crate::async_rc::AsyncRc;
use crate::serve::Server;
use crate::layout::{BatchForgetIn, ForgetOne};

use compio::runtime::spawn;

use std::io::Result;

#[derive(Debug)]
pub struct ForgetReq<'a> {
    pub(super) req: Request,
    pub(super) inodes: &'a [ForgetIno],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForgetIno {
    pub(super) ino: Ino,
    pub(super) nlookup: u64,
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
        self.ino
    }

    pub fn num_lookups(&self) -> u64 {
        self.nlookup
    }
}

impl From<ForgetIno> for Ino {
    fn from(forget: ForgetIno) -> Ino {
        forget.ino
    }
}

impl Server {
    pub fn batch_forget<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        body: &[u8],
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let Some(subhdr) = self.decode::<BatchForgetIn>(body, fs, &req) else {
            return Ok(());
        };

        let count = subhdr.count as usize;

        if count == 0 {
            return Ok(());
        }

        let data = &body[std::mem::size_of::<BatchForgetIn>()..];

        if data.len() < count * std::mem::size_of::<ForgetOne>() {
            let mut tx = self.tx.clone();
            let fs = fs.clone();
            let unique = req.id();
            spawn(async move {
                let _fs = fs;
                let _ = send_error(crate::Error::EINVAL, unique, &mut tx).await;
            }).detach();
            return Ok(());
        }

        todo!();

        Ok(())
    }
}
