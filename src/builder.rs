use crate::MountOpt;
use crate::mount::{DefaultMount, Mount};

use arrayvec::ArrayVec;

use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Builder<M = DefaultMount> {
    mount: M,
    flags: Flags,
    fsname: Option<OsString>,
    subtype: Option<OsString>,
    max_read: Option<usize>,
    blksize: Option<usize>,
    pub(crate) workers: usize,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Flags: u16 {
        const RW = 1 << 0;
        const SUID = 1 << 1;
        const DEV = 1 << 2;
        const EXEC = 1 << 3;
        const ASYNC = 1 << 4;
        const ATIME = 1 << 5;
        const DIRATIME = 1 << 6;
        const LAZYTIME = 1 << 7;
        const RELATIME = 1 << 8;
        const STRICTATIME = 1 << 8;
        const DIRSYNC = 1 << 9;
        const SYMFOLLOW = 1 << 10;
        const ALLOW_OTHER = 1 << 11;
        const ALLOW_ROOT = 1 << 12;
        const DEFAULT_PERMS = 1 << 13;
        const BLKDEV = 1 << 14;
        const LARGE_READ = 1 << 15;

        const DEFAULT = Self::RW.bits()
            | Self::EXEC.bits()
            | Self::ASYNC.bits()
            | Self::ATIME.bits()
            | Self::DIRATIME.bits()
            | Self::SYMFOLLOW.bits();
    }
}

pub(crate) const MAX_OPTS: usize = 19;

pub(crate) type MountOptList = ArrayVec<MountOpt, MAX_OPTS>;

impl Builder {
    pub fn new() -> Self {
        Self::with_mount_method(DefaultMount)
    }
}

impl<M: Mount> Builder<M> {
    pub fn with_mount_method(mount_method: M) -> Self {
        Self {
            mount: mount_method,
            flags: Flags::DEFAULT,
            fsname: None,
            subtype: None,
            max_read: None,
            blksize: None,
            workers: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        }
    }

    pub(crate) fn into_args(self) -> (M, MountOptList, usize) {
        struct Arg(Flags, MountOpt, Option<MountOpt>);

        const ARGS: [Arg; 15] = [
            Arg(Flags::RW, MountOpt::Rw, Some(MountOpt::Ro)),
            Arg(Flags::SUID, MountOpt::Suid, Some(MountOpt::NoSuid)),
            Arg(Flags::DEV, MountOpt::Dev, Some(MountOpt::NoDev)),
            Arg(Flags::EXEC, MountOpt::Exec, Some(MountOpt::NoExec)),
            Arg(Flags::ASYNC, MountOpt::Async, Some(MountOpt::Sync)),
            Arg(Flags::ATIME, MountOpt::Atime, Some(MountOpt::NoAtime)),
            Arg(
                Flags::DIRATIME,
                MountOpt::DirAtime,
                Some(MountOpt::NoDirAtime),
            ),
            Arg(
                Flags::LAZYTIME,
                MountOpt::LazyTime,
                Some(MountOpt::NoLazyTime),
            ),
            Arg(
                Flags::RELATIME,
                MountOpt::RelAtime,
                Some(MountOpt::NoRelAtime),
            ),
            Arg(
                Flags::STRICTATIME,
                MountOpt::StrictAtime,
                Some(MountOpt::NoStrictAtime),
            ),
            Arg(Flags::DIRSYNC, MountOpt::DirSync, None),
            Arg(
                Flags::SYMFOLLOW,
                MountOpt::SymFollow,
                Some(MountOpt::NoSymFollow),
            ),
            Arg(Flags::DEFAULT_PERMS, MountOpt::DefaultPermissions, None),
            Arg(Flags::BLKDEV, MountOpt::BlockDev, None),
            Arg(Flags::LARGE_READ, MountOpt::LargeRead, None),
        ];

        let Self {
            mount,
            flags,
            fsname,
            subtype,
            max_read,
            blksize,
            workers,
        } = self;

        let mut opts = ArrayVec::new();

        for Arg(flag, enable, disable) in ARGS {
            if flags.contains(flag) {
                opts.push(enable);
            } else if let Some(disable) = disable {
                opts.push(disable);
            }
        }

        if flags.contains(Flags::ALLOW_OTHER) {
            opts.push(MountOpt::AllowOther);
        } else if flags.contains(Flags::ALLOW_ROOT) {
            opts.push(MountOpt::AllowRoot);
        }

        if let Some(fsname) = fsname {
            opts.push(MountOpt::FsName(fsname));
        }

        if let Some(subtype) = subtype {
            opts.push(MountOpt::SubType(subtype));
        }

        if let Some(max_read) = max_read {
            opts.push(MountOpt::MaxRead(max_read));
        }

        if let Some(blksize) = blksize {
            opts.push(MountOpt::BlockSize(blksize));
        }

        (mount, opts, workers)
    }

    pub async fn mount<F>(
        self,
        fs: F,
        mountpoint: impl AsRef<Path>,
    ) -> std::io::Result<crate::handle::HandleIter<F::Fs, M::Unmount>>
    where
        F: crate::fs::MountFs<M::Conn>,
    {
        crate::handle::multi_mount(self, fs, mountpoint.as_ref().into()).await
    }

    pub fn mount_blocking<F>(self, fs: F, mountpoint: impl AsRef<Path>) -> std::io::Result<()>
    where
        F: crate::fs::MountFs<M::Conn>,
        M::Unmount: Send,
        M::SharedConn: Send,
        F::Fs: Clone + Send + 'static,
    {
        crate::mount_blocking(self, fs, mountpoint)
    }
}

impl<M> Builder<M> {
    pub fn workers(mut self, num_workers: usize) -> Self {
        self.workers = num_workers;
        self
    }

    pub fn mount_method<T>(self, mount_method: T) -> Builder<T>
    where
        T: Mount,
    {
        let Self {
            flags,
            fsname,
            subtype,
            max_read,
            blksize,
            workers,
            ..
        } = self;

        Builder {
            mount: mount_method,
            flags,
            fsname,
            subtype,
            max_read,
            blksize,
            workers,
        }
    }

    pub fn write(mut self, writeable: bool) -> Self {
        self.flags.set(Flags::RW, writeable);
        self
    }

    pub fn suid(mut self, enable: bool) -> Self {
        self.flags.set(Flags::SUID, enable);
        self
    }

    pub fn dev(mut self, enable: bool) -> Self {
        self.flags.set(Flags::DEV, enable);
        self
    }

    pub fn exec(mut self, enable: bool) -> Self {
        self.flags.set(Flags::EXEC, enable);
        self
    }

    pub fn sync(mut self, enable: bool) -> Self {
        self.flags.set(Flags::ASYNC, !enable);
        self
    }

    pub fn atime(mut self, enable: bool) -> Self {
        self.flags.set(Flags::ATIME, enable);
        self
    }

    pub fn dir_atime(mut self, enable: bool) -> Self {
        self.flags.set(Flags::DIRATIME, enable);
        self
    }

    pub fn lazy_time(mut self, enable: bool) -> Self {
        self.flags.set(Flags::LAZYTIME, enable);
        self
    }

    pub fn relative_atime(mut self, enable: bool) -> Self {
        self.flags.set(Flags::RELATIME, enable);
        self
    }

    pub fn strict_atime(mut self, enable: bool) -> Self {
        self.flags.set(Flags::STRICTATIME, enable);
        self
    }

    pub fn dir_sync(mut self, enable: bool) -> Self {
        self.flags.set(Flags::DIRSYNC, enable);
        self
    }

    pub fn follow_symlinks(mut self, enable: bool) -> Self {
        self.flags.set(Flags::SYMFOLLOW, enable);
        self
    }

    pub fn allow_other(mut self, enable: bool) -> Self {
        self.flags.set(Flags::ALLOW_OTHER, enable);
        self
    }

    pub fn allow_root(mut self, enable: bool) -> Self {
        self.flags.set(Flags::ALLOW_ROOT, enable);
        self
    }

    pub fn default_permissions(mut self, enable: bool) -> Self {
        self.flags.set(Flags::DEFAULT_PERMS, enable);
        self
    }

    pub fn block_device(mut self, enable: bool) -> Self {
        self.flags.set(Flags::BLKDEV, enable);
        self
    }

    pub fn large_read(mut self, enable: bool) -> Self {
        self.flags.set(Flags::LARGE_READ, enable);
        self
    }

    pub fn fs_name(mut self, name: Option<impl Into<OsString>>) -> Self {
        self.fsname = name.map(Into::into);
        self
    }

    pub fn sub_type(mut self, sub_type: Option<impl Into<OsString>>) -> Self {
        self.subtype = sub_type.map(Into::into);
        self
    }

    pub fn max_read(mut self, max_read: impl Into<Option<usize>>) -> Self {
        self.max_read = max_read.into();
        self
    }

    pub fn block_size(mut self, block_size: impl Into<Option<usize>>) -> Self {
        self.blksize = block_size.into();
        self
    }

    pub fn option(self, opt: MountOpt) -> Self {
        match opt {
            MountOpt::Rw => self.write(true),
            MountOpt::Ro => self.write(false),
            MountOpt::Suid => self.suid(true),
            MountOpt::NoSuid => self.suid(false),
            MountOpt::Dev => self.dev(true),
            MountOpt::NoDev => self.dev(false),
            MountOpt::Exec => self.exec(true),
            MountOpt::NoExec => self.exec(false),
            MountOpt::Async => self.sync(false),
            MountOpt::Sync => self.sync(true),
            MountOpt::Atime => self.atime(true),
            MountOpt::NoAtime => self.atime(false),
            MountOpt::DirAtime => self.dir_atime(true),
            MountOpt::NoDirAtime => self.dir_atime(false),
            MountOpt::LazyTime => self.lazy_time(true),
            MountOpt::NoLazyTime => self.lazy_time(false),
            MountOpt::RelAtime => self.relative_atime(true),
            MountOpt::NoRelAtime => self.relative_atime(false),
            MountOpt::StrictAtime => self.strict_atime(true),
            MountOpt::NoStrictAtime => self.strict_atime(false),
            MountOpt::DirSync => self.dir_sync(true),
            MountOpt::SymFollow => self.follow_symlinks(true),
            MountOpt::NoSymFollow => self.follow_symlinks(false),
            MountOpt::AllowOther => self.allow_other(true),
            MountOpt::AllowRoot => self.allow_root(true),
            MountOpt::DefaultPermissions => self.default_permissions(true),
            MountOpt::BlockDev => self.block_device(true),
            MountOpt::LargeRead => self.large_read(true),
            MountOpt::FsName(name) => self.fs_name(Some(name)),
            MountOpt::SubType(subtype) => self.sub_type(Some(subtype)),
            MountOpt::MaxRead(max_read) => self.max_read(max_read),
            MountOpt::BlockSize(blksize) => self.block_size(blksize),
        }
    }

    pub fn options<I>(self, opts: I) -> Self
    where
        I: IntoIterator<Item = MountOpt>,
    {
        opts.into_iter().fold(self, Self::option)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
