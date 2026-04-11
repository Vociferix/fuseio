use super::{Body, Ino, OFlag, OpenReq, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{MsgOut, OpenIn};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

impl Server {
    pub fn opendir<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<OpenIn>(body)?.0;

        let flags = OFlag::from_bits_retain(body.flags.cast_signed());
        if !matches!(
            flags & OFlag::O_ACCMODE,
            OFlag::O_RDONLY | OFlag::O_WRONLY | OFlag::O_RDWR
        ) {
            return Err(Error::EINVAL);
        }

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = OpenReq {
                req,
                ino,
                flags,
                dev: tx.clone(),
            };
            handle_error(match fs.open_dir(ureq).await {
                Ok(resp) => tx.send(MsgOut::new(req.id(), resp.build())).await,
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
