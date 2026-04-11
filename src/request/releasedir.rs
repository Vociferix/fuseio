use super::{Body, FileHandle, Ino, OFlag, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::ReleaseIn;
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct ReleaseDirReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    open_flags: OFlag,
}

impl ReleaseDirReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn open_flags(&self) -> OFlag {
        self.open_flags
    }
}

impl std::ops::Deref for ReleaseDirReq {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn releasedir<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let hdr = decode::<ReleaseIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = ReleaseDirReq {
                req,
                ino,
                fh: FileHandle(hdr.fh),
                open_flags: OFlag::from_bits_retain(hdr.flags.cast_signed()),
            };

            handle_error(send_result(fs.release_dir(ureq).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
