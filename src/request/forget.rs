use super::{Body, ForgetIno, ForgetReq, Ino, Request, decode};
use crate::async_rc::AsyncRc;
use crate::layout::{ForgetIn, ForgetOne};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

impl Server {
    pub fn forget<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<ForgetIn>(body)?.0;

        let fs = fs.clone();

        spawn(async move {
            let inos = [ForgetIno {
                inner: ForgetOne {
                    nodeid: ino.as_raw(),
                    nlookup: body.nlookup,
                },
            }];

            let req = ForgetReq { req, inodes: &inos };

            fs.forget(&req).await;
        })
        .detach();

        Ok(())
    }
}
