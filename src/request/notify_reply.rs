use super::{Body, Ino, Request};
use crate::async_rc::AsyncRc;
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

impl Server {
    pub fn notify_reply<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        Ok(())
    }
}
