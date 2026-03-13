use super::{Ino, Request};
use crate::Filesystem;
use crate::async_rc::AsyncRc;
use crate::serve::Server;

use compio::runtime::spawn;

use std::io::Result;

impl Server {
    pub fn create<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        Ok(())
    }
}
