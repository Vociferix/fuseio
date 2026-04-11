use super::{Body, FileHandle, Ino, LockOwner, OFlag, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::ReleaseIn;
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct ReleaseFlags: u32 {
        const FLUSH = 1 << 0;
        const UNLOCK = 1 << 1;
    }
}

#[derive(Debug)]
pub struct ReleaseFileReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    open_flags: OFlag,
    lock_owner: Option<LockOwner>,
    flush: bool,
}

impl ReleaseFileReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn open_flags(&self) -> OFlag {
        self.open_flags
    }

    pub fn flush(&self) -> bool {
        self.flush
    }

    pub fn release_flock(&self) -> bool {
        self.lock_owner.is_some()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }
}

impl std::ops::Deref for ReleaseFileReq {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl Server {
    pub fn release<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let hdr = decode::<ReleaseIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let rel_flags = ReleaseFlags::from_bits_retain(hdr.release_flags);
            let ureq = ReleaseFileReq {
                req,
                ino,
                fh: FileHandle(hdr.fh),
                open_flags: OFlag::from_bits_retain(hdr.flags.cast_signed()),
                lock_owner: rel_flags
                    .contains(ReleaseFlags::UNLOCK)
                    .then_some(LockOwner(hdr.lock_owner)),
                flush: rel_flags.contains(ReleaseFlags::FLUSH),
            };

            handle_error(send_result(fs.release_file(ureq).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
