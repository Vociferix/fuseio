use super::{Ino, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{BmapIn, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct BmapReq {
    req: Request,
    inode: Ino,
    block: u64,
    blocksize: u32,
}

impl BmapReq {
    pub fn ino(&self) -> Ino {
        self.inode
    }

    pub fn block(&self) -> u64 {
        self.block
    }

    pub fn block_size(&self) -> usize {
        self.blocksize as usize
    }
}

impl std::ops::Deref for BmapReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn bmap<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<BmapIn>(body)?.0;

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = BmapReq {
                req,
                inode: ino,
                block: body.block,
                blocksize: body.blocksize,
            };
            handle_error(match fs.bmap(&req).await {
                Ok(idx) => tx.send(MsgOut::new(req.id(), idx)).await,
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
