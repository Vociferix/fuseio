use super::{Ino, Request};
use crate::async_rc::AsyncRc;
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

#[cfg(target_os = "macos")]
impl Server {
    pub fn setvolname<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        Ok(())
    }
}
