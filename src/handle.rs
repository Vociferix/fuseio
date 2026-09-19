use crate::async_arc::AsyncArc;
use crate::builder::MountOptList;
use crate::dev_fuse::{DevFuse, FuseChannel};
use crate::handshake::{Config, Init, handshake};
use crate::server::{Message, Server};
use crate::types::{ReplyInitFlags, Version};
use crate::{
    Builder, MountOpt,
    fs::{BindFs, Fs, MountFs},
    mount::{Mount, Unmount},
};

use crossfire::{
    AsyncRx, MAsyncTx,
    mpsc::{Array, bounded_async},
};

use std::io::Result;
use std::mem::ManuallyDrop;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Handle<F, U> {
    pub(crate) id: usize,
    pub(crate) dev: DevFuse,
    pub(crate) fs: F,
    pub(crate) minor_ver: u32,
    pub(crate) flags: ReplyInitFlags,
    pub(crate) max_readahead: usize,
    pub(crate) mesh_rx: AsyncRx<Array<Message>>,
    pub(crate) mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
    pub(crate) config: Config,
    pub(crate) once: Option<Once<U>>,
}

#[derive(Clone)]
pub struct UnmountHandle {
    mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
}

pub struct HandleIter<F, U: Unmount> {
    id: usize,
    devs: std::vec::IntoIter<(DevFuse, AsyncRx<Array<Message>>)>,
    fs: ManuallyDrop<F>,
    minor_ver: u32,
    flags: ReplyInitFlags,
    max_readahead: usize,
    mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
    config: Config,
    once: Option<Once<U>>,
}

pub(crate) struct Once<U> {
    pub(crate) unmount: U,
    pub(crate) path: PathBuf,
    pub(crate) opts: MountOptList,
}

pub async fn multi_mount<M, F>(
    builder: Builder<M>,
    fs: F,
    mountpoint: PathBuf,
) -> Result<HandleIter<F::Fs, M::Unmount>>
where
    M: Mount,
    F: MountFs,
{
    let (mounter, dev, opts, workers) = builder.into_args();

    if workers == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cannot mount with zero workers",
        ));
    }

    let dev = AsyncArc::new(DevFuse::open(dev).await?);

    let unmounter = mounter
        .mount(dev.as_fd(), &mountpoint, opts.as_ref())
        .await?;

    let Init {
        fs,
        ver,
        flags,
        max_readahead,
        config,
    } = match handshake(fs, dev.clone(), opts.as_ref()).await {
        Ok(init) => init,
        Err(err) => {
            let _ = unmounter
                .unmount(dev.as_fd(), mountpoint.as_ref(), opts.as_ref())
                .await;
            return Err(err);
        }
    };

    let dev = AsyncArc::unwrap(dev).await;

    let mut devs = Vec::with_capacity(workers);
    let mut mesh_tx = Vec::with_capacity(workers);
    for _ in 1..workers {
        let dev = match dev.try_clone().await {
            Ok(dev) => dev,
            Err(err) => {
                let _ = unmounter
                    .unmount(dev.as_fd(), mountpoint.as_ref(), opts.as_ref())
                    .await;
                return Err(err);
            }
        };
        let (tx, rx) = bounded_async(workers * 4);
        devs.push((dev, rx));
        mesh_tx.push(tx);
    }
    let (tx, rx) = bounded_async(workers * 4);
    devs.push((dev, rx));
    mesh_tx.push(tx);

    Ok(HandleIter {
        id: 0,
        devs: devs.into_iter(),
        fs: ManuallyDrop::new(fs),
        minor_ver: ver.1,
        flags,
        max_readahead,
        mesh_tx: mesh_tx.into(),
        config,
        once: Some(Once {
            unmount: unmounter,
            path: mountpoint,
            opts,
        }),
    })
}

impl<F, U> Handle<F, U>
where
    F: BindFs,
    U: Unmount,
{
    pub async fn bind_and_serve(self) -> Result<()> {
        let server = Server::new(self).await?;
        Server::serve_requests(&server).await;
        Server::unmount(server).await
    }

    pub fn unmount_handle(&self) -> UnmountHandle {
        UnmountHandle {
            mesh_tx: self.mesh_tx.clone(),
        }
    }
}

impl UnmountHandle {
    pub async fn unmount(&self) {
        for tx in self.mesh_tx.iter() {
            let _ = tx.send(Message::Shutdown).await;
        }
    }
}

impl<F, U> Iterator for HandleIter<F, U>
where
    F: Clone,
    U: Unmount,
{
    type Item = Handle<F, U>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.id;
        self.id += 1;

        let (dev, rx) = self.devs.next()?;

        let fs = if self.devs.len() == 0 {
            unsafe { ManuallyDrop::take(&mut self.fs) }
        } else {
            (*self.fs).clone()
        };

        Some(Handle {
            id,
            dev,
            fs,
            minor_ver: self.minor_ver,
            flags: self.flags,
            max_readahead: self.max_readahead,
            mesh_rx: rx,
            mesh_tx: self.mesh_tx.clone(),
            config: self.config,
            once: self.once.take(),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.devs.size_hint()
    }
}

impl<F, U> ExactSizeIterator for HandleIter<F, U>
where
    F: Clone,
    U: Unmount,
{
    fn len(&self) -> usize {
        self.devs.len()
    }
}

impl<F, U> std::iter::FusedIterator for HandleIter<F, U>
where
    F: Clone,
    U: Unmount,
{
}

impl<F, U> Drop for HandleIter<F, U>
where
    U: Unmount,
{
    fn drop(&mut self) {
        if let Some((dev, _)) = self.devs.next() {
            unsafe {
                ManuallyDrop::drop(&mut self.fs);
            }

            if let Some(once) = self.once.take() {
                compio::runtime::spawn(async move {
                    let _ = once
                        .unmount
                        .unmount(dev.as_fd(), &once.path, &once.opts)
                        .await;
                })
                .detach();
            }
        }
    }
}
