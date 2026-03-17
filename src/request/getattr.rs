use super::{
    Body, FileFlag, FileHandle, Gid, Ino, Mode, Request, Uid, decode, handle_error, send_error,
};
use crate::async_rc::AsyncRc;
use crate::layout::{Attr, AttrOut, GetAttrFlags, GetAttrIn, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct GetAttrsReq {
    req: Request,
    ino: Ino,
    fh: Option<FileHandle>,
}

#[derive(Debug)]
pub struct GetAttrsResp {
    attrs: InodeAttrs,
    ttl: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
pub struct InodeAttrs {
    size: u64,
    blocks: Option<u64>,
    atime: SystemTime,
    mtime: SystemTime,
    ctime: SystemTime,
    #[cfg(target_os = "macos")]
    crtime: SystemTime,
    mode: Mode,
    nlink: usize,
    uid: Uid,
    gid: Gid,
    rdev: u32,
    #[cfg(target_os = "macos")]
    flags: u32,
    blksize: usize,
}

impl GetAttrsReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.fh
    }
}

impl std::ops::Deref for GetAttrsReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl GetAttrsResp {
    pub fn new(attrs: InodeAttrs, ttl: Duration) -> Self {
        Self {
            attrs,
            ttl: Some(ttl),
        }
    }

    fn build(self, ino: Ino) -> AttrOut {
        let (attr_valid, attr_valid_nsec) = if let Some(ttl) = self.ttl {
            (ttl.as_secs(), ttl.subsec_nanos())
        } else {
            (u64::MAX, u32::MAX)
        };

        AttrOut {
            attr_valid,
            attr_valid_nsec,
            dummy: 0,
            attr: self.attrs.build(ino),
        }
    }
}

impl From<InodeAttrs> for GetAttrsResp {
    fn from(attrs: InodeAttrs) -> Self {
        Self { attrs, ttl: None }
    }
}

impl InodeAttrs {
    pub fn new() -> Self {
        Self {
            size: 0,
            blocks: None,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            #[cfg(target_os = "macos")]
            crtime: SystemTime::UNIX_EPOCH,
            mode: Mode::empty(),
            nlink: 0,
            uid: Uid::from_raw(0),
            gid: Gid::from_raw(0),
            rdev: 0,
            #[cfg(target_os = "macos")]
            flags: FileFlag::empty(),
            blksize: 4096,
        }
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn blocks(mut self, blocks: u64) -> Self {
        self.blocks = Some(blocks);
        self
    }

    pub fn atime(mut self, atime: SystemTime) -> Self {
        self.atime = atime;
        self
    }

    pub fn mtime(mut self, mtime: SystemTime) -> Self {
        self.mtime = mtime;
        self
    }

    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        self.ctime = ctime;
        self
    }

    pub fn crtime(self, crtime: SystemTime) -> Self {
        #[cfg(target_os = "macos")]
        {
            let mut this = self;
            this.crtime = crtime;
            this
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = crtime;
            self
        }
    }

    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn hard_links(mut self, count: usize) -> Self {
        self.nlink = count;
        self
    }

    pub fn uid(mut self, uid: Uid) -> Self {
        self.uid = uid;
        self
    }

    pub fn gid(mut self, gid: Gid) -> Self {
        self.gid = gid;
        self
    }

    pub fn device_id(mut self, dev_id: u32) -> Self {
        self.rdev = dev_id;
        self
    }

    pub fn flags(self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            let mut this = self;
            this.flags = flags;
            this
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
            self
        }
    }

    pub fn block_size(mut self, size: usize) -> Self {
        self.blksize = size;
        self
    }

    pub(super) fn build(self, ino: Ino) -> Attr {
        let atime = self
            .atime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        let mtime = self
            .mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        let ctime = self
            .ctime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        #[allow(unused)]
        let crtime: Duration;
        #[cfg(target_os = "macos")]
        {
            crtime = self
                .crtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO);
        }
        Attr {
            ino: ino.as_raw(),
            size: self.size,
            block: self.blocks.unwrap_or_else(|| (self.size + 511) / 512),
            atime: atime.as_secs(),
            mtime: mtime.as_secs(),
            ctime: ctime.as_secs(),
            #[cfg(target_os = "macos")]
            crtime: crtime.as_secs(),
            atimensec: atime.subsec_nanos(),
            mtimensec: mtime.subsec_nanos(),
            ctimensec: ctime.subsec_nanos(),
            #[cfg(target_os = "macos")]
            crtimensec: crtime.subsec_nanos(),
            mode: self.mode.bits(),
            nlink: self.nlink.try_into().unwrap_or(u32::MAX),
            uid: self.uid.as_raw(),
            gid: self.gid.as_raw(),
            rdev: self.rdev,
            #[cfg(target_os = "macos")]
            flags: self.flags.bits(),
            blksize: self.blksize.try_into().unwrap_or(4096),
            padding: 0,
        }
    }
}

impl Default for InodeAttrs {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn getattr<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<GetAttrIn>(body)?.0;

        let ino = if ino.as_raw() == 1 && !self.root_ino_init.replace(true) {
            self.root_ino
        } else {
            ino
        };

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let minor = self.ver.1;

        spawn(async move {
            let fh = if body.getattr_flags.contains(GetAttrFlags::HAVE_FH) {
                Some(FileHandle(body.fh))
            } else {
                None
            };

            let req = GetAttrsReq { req, ino, fh };

            handle_error(match fs.get_attrs(&req).await {
                Ok(resp) => {
                    let body = resp.build(ino);
                    if minor < 9 {
                        tx.send(MsgOut::new(req.id(), body.compat())).await
                    } else {
                        tx.send(MsgOut::new(req.id(), body)).await
                    }
                }
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
