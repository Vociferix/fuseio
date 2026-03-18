use crate::BufPool;
use crate::async_rc::AsyncRc;
use crate::channel::{Receiver, Sender, channel};
use crate::layout::{
    self, HeaderIn, HeaderOut, InitIn, InitOut, InitOutCompat, InitOutCompat22, Opcode,
};
use crate::mount::{Mount, Unmount};
use crate::request::Body;
use crate::request::Ino;
use crate::{Filesystem, KernelConfig, MountHandle, MountOpt, Version};

use bytemuck::{Zeroable, bytes_of_mut};
use compio::runtime::event::{Event, EventHandle};
use futures_util::{FutureExt, select_biased};

use std::cell::Cell;
use std::io::{Error as IoError, ErrorKind, Result};
use std::os::fd::AsFd;

pub struct Server {
    pub(crate) tx: Sender,
    pub(crate) bufs: BufPool,
    pub(crate) ver: Version,
    pub(crate) root_ino: Ino,
    pub(crate) root_ino_init: Cell<bool>,
}

pub async fn mount(
    builder: crate::Builder<impl Mount>,
    mut fs: impl Filesystem,
    mountpoint: impl AsRef<std::path::Path>,
) -> Result<MountHandle> {
    let (mounter, dev, opts) = builder.into_args();

    let (mut tx, mut rx) = channel(dev).await?;

    let unmounter = mounter
        .mount(tx.as_fd(), mountpoint.as_ref(), opts.as_ref())
        .await?;

    let (version, root_ino) = match handshake(&mut fs, &mut tx, &mut rx, opts.as_ref()).await {
        Ok(init) => init,
        Err(err) => {
            let _ = unmounter.unmount(tx.as_fd(), mountpoint.as_ref(), opts.as_ref());
            return Err(err);
        }
    };

    let cancel_signal = Event::new();
    let cancel_notifier = cancel_signal.handle();
    let unmount_signal = Event::new();
    let unmount_notifier = unmount_signal.handle();
    let error_unmount = unmount_signal.handle();

    let mountpoint = mountpoint.as_ref().to_path_buf();
    let fd = tx.clone();
    let unmount_future = compio::runtime::spawn(async move {
        select_biased! {
            () = cancel_signal.wait().fuse() => Ok(()),
            () = unmount_signal.wait().fuse() => unmounter.unmount(fd.as_fd(), &mountpoint, opts.as_ref()).await,
        }
    })
        .map(|res| match res {
            Ok(res) => res,
            Err(panic) => std::panic::resume_unwind(panic),
        });

    let server = Server {
        tx,
        bufs: BufPool::new(),
        ver: version,
        root_ino,
        root_ino_init: Cell::new(root_ino.as_raw() == 1),
    };

    let task = compio::runtime::spawn(Server::serve(
        server,
        rx,
        fs,
        unmount_future,
        cancel_notifier,
        error_unmount,
    ));

    Ok(MountHandle::new(task, unmount_notifier))
}

const HDR_LEN: usize = std::mem::size_of::<HeaderIn>();

impl Server {
    async fn serve<F, U>(
        mut self,
        mut rx: Receiver,
        fs: F,
        unmount: U,
        cancel_notifier: EventHandle,
        error_notifier: EventHandle,
    ) -> Result<()>
    where
        F: Filesystem,
        U: Future<Output = Result<()>>,
    {
        let fs = AsyncRc::new(fs);

        let mut hdr = HeaderIn::zeroed();

        loop {
            match self.serve_one(&mut rx, &fs, &mut hdr).await {
                Ok(true) => break,
                Ok(false) => {}
                Err(err) => {
                    error_notifier.notify();
                    let _ = unmount.await;
                    let mut fs = AsyncRc::unwrap(fs).await;
                    let _ = fs.shutdown().await;
                    return Err(err);
                }
            }
        }

        cancel_notifier.notify();

        let mut fs = AsyncRc::unwrap(fs).await;

        if let Err(err) = fs.shutdown().await {
            let _ = unmount.await;
            return Err(err.into());
        }

        unmount.await
    }

    async fn serve_one<F>(
        &mut self,
        rx: &mut Receiver,
        fs: &AsyncRc<F>,
        hdr: &mut HeaderIn,
    ) -> Result<bool>
    where
        F: Filesystem,
    {
        let msg = match rx.recv().await {
            Ok(msg) if msg.len() < HDR_LEN => {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "invalid request from kernel",
                ));
            }
            Ok(msg) => msg,
            Err(err) => return Err(err),
        };

        *hdr = unsafe { std::ptr::read(msg.as_ptr() as *const HeaderIn) };

        if msg.len() < hdr.len as usize {
            let unique = hdr.unique;
            let fs = fs.clone();
            let mut tx = self.tx.clone();
            compio::runtime::spawn(async move {
                let _fs = fs;
                let res = tx
                    .send(HeaderOut {
                        len: const { std::mem::size_of::<HeaderOut>() as u32 },
                        error: -crate::Error::EPROTO.raw_os_error(),
                        unique,
                    })
                    .await;
                if let Err(err) = res {
                    log::error!("failed to send error response to kernel: {err}");
                }
            })
            .detach();
            return Ok(false);
        }

        let body = Body::new(msg, HDR_LEN..(hdr.len as usize));

        match self.dispatch(fs, &hdr, body).await {
            Ok(destroy) => Ok(destroy),
            Err(err) => {
                let unique = hdr.unique;
                let fs = fs.clone();
                let mut tx = self.tx.clone();
                compio::runtime::spawn(async move {
                    let _fs = fs;
                    let _ = tx
                        .send(HeaderOut {
                            len: const { std::mem::size_of::<HeaderOut>() as u32 },
                            error: -err.raw_os_error(),
                            unique,
                        })
                        .await;
                })
                .detach();
                return Ok(false);
            }
        }
    }

    async fn dispatch<F>(
        &mut self,
        fs: &AsyncRc<F>,
        hdr: &HeaderIn,
        body: Body,
    ) -> std::result::Result<bool, crate::Error>
    where
        F: Filesystem,
    {
        let Some(ino) = crate::request::Ino::from_raw(hdr.nodeid) else {
            let unique = hdr.unique;
            let fs = fs.clone();
            let mut tx = self.tx.clone();
            compio::runtime::spawn(async move {
                let _fs = fs;
                let _ = tx
                    .send(HeaderOut {
                        len: const { std::mem::size_of::<HeaderOut>() as u32 },
                        error: -crate::Error::EINVAL.raw_os_error(),
                        unique,
                    })
                    .await;
            })
            .detach();
            return Ok(false);
        };

        let req = crate::request::Request::new(hdr);

        match hdr.opcode {
            Opcode::LOOKUP => self.lookup(fs, req, ino, body),
            Opcode::FORGET => self.forget(fs, req, ino, body),
            Opcode::GETATTR => self.getattr(fs, req, ino, body),
            Opcode::SETATTR => self.setattr(fs, req, ino, body),
            Opcode::READLINK => self.readlink(fs, req, ino, body),
            Opcode::SYMLINK => self.symlink(fs, req, ino, body),
            Opcode::MKNOD => self.mknod(fs, req, ino, body),
            Opcode::MKDIR => self.mkdir(fs, req, ino, body),
            Opcode::UNLINK => self.unlink(fs, req, ino, body),
            Opcode::RMDIR => self.rmdir(fs, req, ino, body),
            Opcode::RENAME => self.rename(fs, req, ino, body),
            Opcode::LINK => self.link(fs, req, ino, body),
            Opcode::OPEN => self.open(fs, req, ino, body),
            Opcode::READ => self.read(fs, req, ino, body),
            Opcode::WRITE => self.write(fs, req, ino, body),
            Opcode::STATFS => self.statfs(fs, req, ino, body),
            Opcode::RELEASE => self.release(fs, req, ino, body),
            Opcode::FSYNC => self.fsync(fs, req, ino, body),
            Opcode::SETXATTR => self.setxattr(fs, req, ino, body),
            Opcode::GETXATTR => self.getxattr(fs, req, ino, body),
            Opcode::LISTXATTR => self.listxattr(fs, req, ino, body),
            Opcode::REMOVEXATTR => self.removexattr(fs, req, ino, body),
            Opcode::FLUSH => self.flush(fs, req, ino, body),
            //Opcode::INIT => {}, // handled in handshake
            Opcode::OPENDIR => self.opendir(fs, req, ino, body),
            Opcode::READDIR => self.readdir(fs, req, ino, body),
            Opcode::RELEASEDIR => self.releasedir(fs, req, ino, body),
            Opcode::FSYNCDIR => self.fsyncdir(fs, req, ino, body),
            Opcode::GETLK => self.getlk(fs, req, ino, body),
            Opcode::SETLK => self.setlk(fs, req, ino, body),
            Opcode::SETLKW => self.setlkw(fs, req, ino, body),
            Opcode::ACCESS => self.access(fs, req, ino, body),
            Opcode::CREATE => self.create(fs, req, ino, body),
            Opcode::INTERRUPT => self.interrupt(fs, req, ino, body),
            Opcode::BMAP => self.bmap(fs, req, ino, body),
            Opcode::DESTROY => return self.destroy(hdr).await,
            Opcode::IOCTL => self.ioctl(fs, req, ino, body),
            Opcode::POLL => self.poll(fs, req, ino, body),
            Opcode::NOTIFY_REPLY => self.notify_reply(fs, req, ino, body),
            Opcode::BATCH_FORGET => self.batch_forget(fs, req, ino, body),
            Opcode::FALLOCATE => self.fallocate(fs, req, ino, body),
            Opcode::READDIRPLUS => self.readdirplus(fs, req, ino, body),
            Opcode::RENAME2 => self.rename2(fs, req, ino, body),
            Opcode::LSEEK => self.lseek(fs, req, ino, body),
            Opcode::COPY_FILE_RANGE => self.copy_file_range(fs, req, ino, body),

            #[cfg(target_os = "macos")]
            Opcode::SETVOLNAME => self.setvolname(fs, req, ino, body),
            #[cfg(target_os = "macos")]
            Opcode::GETXTIMES => self.getxtimes(fs, req, ino, body),
            #[cfg(target_os = "macos")]
            Opcode::EXCHANGE => self.exchange(fs, req, ino, body),

            // TODO: Is this a normal opcode to setup a CUSE device or does this
            //       come in instead of Opcode::INIT in the case of CUSE?
            //Opcode::CUSE_INIT => {},
            _ => return Err(crate::Error::EINVAL),
        }?;

        Ok(false)
    }

    async fn destroy(&mut self, hdr: &HeaderIn) -> std::result::Result<bool, crate::Error> {
        self.tx
            .send(HeaderOut {
                len: const { std::mem::size_of::<HeaderOut>() as u32 },
                error: 0,
                unique: hdr.unique,
            })
            .await?;
        Ok(true)
    }
}

async fn handshake(
    fs: &mut impl Filesystem,
    tx: &mut Sender,
    rx: &mut Receiver,
    opts: &[MountOpt],
) -> Result<(Version, Ino)> {
    let mut hdr = HeaderIn::zeroed();
    let mut init = InitIn::zeroed();

    loop {
        let msg = match rx.recv().await {
            Ok(msg) if msg.len() < HDR_LEN => {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "invalid initialization request from kernel",
                ));
            }
            Ok(msg) => msg,
            Err(err) => return Err(err),
        };

        bytes_of_mut(&mut hdr).copy_from_slice(&msg[..HDR_LEN]);
        let body = &msg[HDR_LEN..];

        if hdr.opcode != Opcode::INIT || body.len() < std::mem::offset_of!(InitIn, flags2) {
            let out = HeaderOut {
                len: const { HDR_LEN as u32 },
                error: -crate::Error::EIO.raw_os_error(),
                unique: hdr.unique,
            };
            let _ = tx.send(out).await;
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "invalid initialization request from kernel",
            ));
        }

        let len = body.len().min(std::mem::size_of::<InitIn>());

        bytes_of_mut(&mut init).copy_from_slice(&body[..len]);

        if init.major <= layout::VERSION_MAJOR {
            break;
        }

        let mut out = layout::MsgOut {
            hdr: HeaderOut {
                len: const { (HDR_LEN + std::mem::size_of::<InitOut>()) as u32 },
                error: 0,
                unique: hdr.unique,
            },
            body: InitOut::zeroed(),
        };

        out.body.major = layout::VERSION_MAJOR;
        out.body.minor = layout::VERSION_MINOR;

        tx.send(out).await?;
    }

    if init.major < layout::VERSION_MAJOR {
        let out = HeaderOut {
            len: const { HDR_LEN as u32 },
            error: -crate::Error::EPROTO.raw_os_error(),
            unique: hdr.unique,
        };
        let _ = tx.send(out).await;
        return Err(IoError::new(
            ErrorKind::Unsupported,
            format!(
                "unsupported FUSE ABI version: {}.{}",
                init.major, init.minor
            ),
        ));
    }

    let conf = match fs.initialize(KernelConfig::new(init), opts).await {
        Ok(conf) => conf,
        Err(err) => {
            let out = HeaderOut {
                len: const { HDR_LEN as u32 },
                error: -err.raw_os_error(),
                unique: hdr.unique,
            };
            let _ = tx.send(out).await;
            return Err(err.into());
        }
    };

    let root_ino = conf.root_inode;

    let body = conf.build();

    if init.minor < 5 {
        let out = layout::MsgOut {
            hdr: HeaderOut {
                len: const { (HDR_LEN + std::mem::size_of::<InitOutCompat>()) as u32 },
                error: 0,
                unique: hdr.unique,
            },
            body: body.compat(),
        };

        tx.send(out).await?;
    } else if init.minor < 23 {
        let out = layout::MsgOut {
            hdr: HeaderOut {
                len: const { (HDR_LEN + std::mem::size_of::<InitOutCompat22>()) as u32 },
                error: 0,
                unique: hdr.unique,
            },
            body: body.compat22(),
        };

        tx.send(out).await?;
    } else {
        let out = layout::MsgOut {
            hdr: HeaderOut {
                len: const { (HDR_LEN + std::mem::size_of::<InitOut>()) as u32 },
                error: 0,
                unique: hdr.unique,
            },
            body,
        };

        tx.send(out).await?;
    }

    Ok((Version(init.major, init.minor), root_ino))
}
