use super::{Entry, Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

impl Server {
    pub fn mknod<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        Ok(())
    }
}
