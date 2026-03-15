use super::{Ino, InodeAttrs, Request, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{EntryOut, MsgOut};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

#[derive(Debug)]
pub struct LookupReq<'a> {
    req: Request,
    parent: Option<Ino>,
    name: &'a OsStr,
}

#[derive(Debug)]
pub struct Entry {
    ino: Ino,
    generation: u64,
    entry_ttl: Option<Duration>,
    attr_ttl: Option<Duration>,
    attr: Option<InodeAttrs>,
}

impl<'a> LookupReq<'a> {
    pub fn parent_ino(&self) -> Option<Ino> {
        self.parent
    }

    pub fn name(&self) -> &'a OsStr {
        self.name
    }
}

impl std::ops::Deref for LookupReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Entry {
    pub fn new(ino: Ino) -> Self {
        Self {
            ino,
            generation: 0,
            entry_ttl: None,
            attr_ttl: None,
            attr: None,
        }
    }

    pub fn generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    pub fn entry_ttl(mut self, ttl: Duration) -> Self {
        self.entry_ttl = Some(ttl);
        self
    }

    pub fn attrs(mut self, attrs: InodeAttrs) -> Self {
        self.attr = Some(attrs);
        self
    }

    pub fn attrs_ttl(mut self, ttl: Duration) -> Self {
        self.attr_ttl = Some(ttl);
        self
    }

    pub(super) fn build(self) -> EntryOut {
        let (entry_valid, entry_valid_nsec) = if let Some(entry_ttl) = self.entry_ttl {
            (entry_ttl.as_secs(), entry_ttl.subsec_nanos())
        } else {
            (u64::MAX, u32::MAX)
        };
        let (attr_valid, attr_valid_nsec) = if let Some(attr_ttl) = self.attr_ttl {
            (attr_ttl.as_secs(), attr_ttl.subsec_nanos())
        } else {
            (u64::MAX, u32::MAX)
        };
        EntryOut {
            nodeid: self.ino.as_raw(),
            generation: self.generation,
            entry_valid,
            attr_valid,
            entry_valid_nsec,
            attr_valid_nsec,
            attr: self.attr.unwrap_or_else(InodeAttrs::new).build(self.ino),
        }
    }
}

impl Server {
    pub fn lookup<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: &[u8]) -> Result<()>
    where
        F: Filesystem,
    {
        let name_len = memchr::memchr(0, body).unwrap_or(body.len());
        let mut buf = self.bufs.checkout_with_capacity::<u8>(name_len);
        buf.extend_from_slice(&body[..name_len]);

        let fs = fs.clone();
        let mut tx = self.tx.clone();
        let minor = self.ver.1;

        let is_root_name = name_len == 0 || buf.as_slice() == b"/";
        let root_is_init = self.root_ino_init.get();

        let root_ino = self.root_ino;

        let ino = if ino.as_raw() == 1 && is_root_name && !root_is_init {
            self.root_ino_init.set(true);
            buf.clear();
            None
        } else if ino == self.root_ino && root_is_init {
            buf.clear();
            None
        } else {
            Some(ino)
        };

        spawn(async move {
            let req = LookupReq {
                req,
                parent: ino,
                name: OsStr::from_bytes(&buf),
            };
            handle_error(match fs.lookup(&req).await {
                Ok(mut resp) => {
                    if ino.is_none() {
                        resp.ino = root_ino;
                    }
                    let body = resp.build();
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
