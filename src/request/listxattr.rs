use super::{Body, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{GetXattrIn, GetXattrOut, HeaderOut, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

const OUT_HDRS_LEN: usize = std::mem::size_of::<HeaderOut>() + std::mem::size_of::<GetXattrOut>();

#[derive(Debug)]
pub struct GetXattrReq<'a> {
    req: Request,
    ino: Ino,
    key: &'a OsStr,
    #[cfg(target_os = "macos")]
    pos: usize,
}

impl Server {
    pub fn listxattr<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        Ok(())
    }
}
