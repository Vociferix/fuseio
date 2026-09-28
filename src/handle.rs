use crate::builder::MountOptList;
use crate::conn::{Conn, ConnectionMeta, SharedConnection};
use crate::handshake::{Config, Init, handshake};
use crate::server::{Message, Server};
use crate::types::ReplyInitFlags;
use crate::{
    Builder,
    fs::{BindFs, MountFs},
    mount::{Mount, Unmount},
};

use crossfire::{
    AsyncRx, MAsyncTx,
    mpsc::{Array, bounded_async},
};

use std::io::Result;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::sync::Arc;

pub struct Handle<F, U: Unmount> {
    pub(crate) id: usize,
    pub(crate) conn: U::SharedConn,
    pub(crate) fs: F,
    pub(crate) minor_ver: u32,
    pub(crate) flags: ReplyInitFlags,
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
    conns: std::vec::IntoIter<(U::SharedConn, AsyncRx<Array<Message>>)>,
    fs: ManuallyDrop<F>,
    minor_ver: u32,
    flags: ReplyInitFlags,
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
    F: MountFs<M::Conn>,
{
    let (mounter, opts, workers) = builder.into_args();

    if workers == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cannot mount with zero workers",
        ));
    }

    let (conn, unmounter) = mounter.mount(&mountpoint, opts.as_ref(), workers).await?;

    // Ensure the connection supports at least sending an error (response header only)
    if conn.max_response_size() < crate::proto::MIN_MSG_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "connection max response size too small",
        ));
    }

    let (init_res, unmounter) = {
        let mut handshake_fut = std::pin::pin!(handshake(fs, &conn, opts.as_ref()));
        let mut unmounter_fut = std::pin::pin!(unmounter);

        match crate::select_biased(unmounter_fut.as_mut(), handshake_fut.as_mut()).await {
            either::Left(Err(err)) => return Err(err),
            either::Left(Ok(unmounter)) => (handshake_fut.await, unmounter),
            either::Right(init_res) => (init_res, unmounter_fut.await?),
        }
    };

    let Init {
        fs,
        ver,
        flags,
        config,
    } = match init_res {
        Ok(init) => init,
        Err(err) => {
            let _ = unmounter
                .unmount(Conn::Shared(conn), mountpoint.as_ref(), opts.as_ref())
                .await;
            return Err(err);
        }
    };

    let mut conns = Vec::with_capacity(workers);
    let mut mesh_tx = Vec::with_capacity(workers);
    for _ in 1..workers {
        let conn = match conn.try_clone().await {
            Ok(conn) => conn,
            Err(err) => {
                let _ = unmounter
                    .unmount(Conn::Shared(conn), mountpoint.as_ref(), opts.as_ref())
                    .await;
                return Err(err);
            }
        };
        let (tx, rx) = bounded_async(workers * 4);
        conns.push((conn, rx));
        mesh_tx.push(tx);
    }
    let (tx, rx) = bounded_async(workers * 4);
    conns.push((conn, rx));
    mesh_tx.push(tx);

    Ok(HandleIter {
        id: 0,
        conns: conns.into_iter(),
        fs: ManuallyDrop::new(fs),
        minor_ver: ver.1,
        flags,
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
    F: BindFs<U::Conn>,
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

        let (conn, rx) = self.conns.next()?;

        let fs = if self.conns.len() == 0 {
            unsafe { ManuallyDrop::take(&mut self.fs) }
        } else {
            (*self.fs).clone()
        };

        Some(Handle {
            id,
            conn,
            fs,
            minor_ver: self.minor_ver,
            flags: self.flags,
            mesh_rx: rx,
            mesh_tx: self.mesh_tx.clone(),
            config: self.config,
            once: self.once.take(),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.conns.size_hint()
    }
}

impl<F, U> ExactSizeIterator for HandleIter<F, U>
where
    F: Clone,
    U: Unmount,
{
    fn len(&self) -> usize {
        self.conns.len()
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
        if let Some((conn, _)) = self.conns.next() {
            unsafe {
                ManuallyDrop::drop(&mut self.fs);
            }

            if let Some(once) = self.once.take() {
                compio::runtime::spawn(async move {
                    let _ = once
                        .unmount
                        .unmount(Conn::Shared(conn), &once.path, &once.opts)
                        .await;
                })
                .detach();
            }
        }
    }
}
