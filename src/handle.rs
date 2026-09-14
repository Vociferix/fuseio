use crate::async_arc::AsyncArc;
use crate::builder::MountOptList;
use crate::dev_fuse::{DevFuse, FuseChannel};
use crate::handshake::{Config, Init, handshake};
use crate::types::{ReplyInitFlags, Version};
use crate::{
    BindFs, Builder, Fs, MountFs, MountOpt,
    mount::{Mount, Unmount},
};

use std::io::Result;
use std::mem::ManuallyDrop;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

pub struct Handle<F, U> {
    dev: DevFuse,
    fs: F,
    minor_ver: u32,
    flags: ReplyInitFlags,
    max_readahead: usize,
    config: Config,
    once: Option<Once<U>>,
}

pub struct HandleIter<F, U: Unmount> {
    devs: std::vec::IntoIter<DevFuse>,
    fs: ManuallyDrop<F>,
    minor_ver: u32,
    flags: ReplyInitFlags,
    max_readahead: usize,
    config: Config,
    once: Option<Once<U>>,
}

struct Once<U> {
    unmount: U,
    path: PathBuf,
    opts: MountOptList,
}

pub async fn multi_mount<M, F>(
    builder: Builder<M>,
    fs: F,
    mountpoint: PathBuf,
    workers: usize,
) -> Result<HandleIter<F::Fs, M::Unmount>>
where
    M: Mount,
    F: MountFs,
{
    if workers == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cannot mount with zero workers",
        ));
    }

    let (mounter, dev, opts) = builder.into_args();

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
        devs.push(dev);
    }
    devs.push(dev);

    Ok(HandleIter {
        devs: devs.into_iter(),
        fs: ManuallyDrop::new(fs),
        minor_ver: ver.1,
        flags,
        max_readahead,
        config,
        once: Some(Once {
            unmount: unmounter,
            path: mountpoint,
            opts,
        }),
    })
}

impl<F, U> Iterator for HandleIter<F, U>
where
    F: Clone,
    U: Unmount,
{
    type Item = Handle<F, U>;

    fn next(&mut self) -> Option<Self::Item> {
        let dev = self.devs.next()?;

        let fs = if self.devs.len() == 0 {
            unsafe { ManuallyDrop::take(&mut self.fs) }
        } else {
            (*self.fs).clone()
        };

        Some(Handle {
            dev,
            fs,
            minor_ver: self.minor_ver,
            flags: self.flags,
            max_readahead: self.max_readahead,
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
        if let Some(dev) = self.devs.next() {
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
