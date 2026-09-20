use super::Req;
use crate::buf::Buf;
use crate::proto::request::ReadDir;
use crate::proto::response::Data;
use crate::types::{FileHandle, Ino, InodeKind, LockOwner, OFlag, SFlag};
use crate::{Error, Result};

use futures_util::{Stream, StreamExt};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct ReadDirReq {
    req: Req,
    readdir: ReadDir,
}

#[derive(Debug)]
pub struct DirEntryBuf {
    max_len: usize,
    cookie: u64,
    count: usize,
    buf: Buf,
}

#[derive(Debug, Clone, Copy)]
pub struct DirEntry<T> {
    pub ino: Ino,

    pub kind: InodeKind,

    /// Arbitrary locator for the _next_ entry.
    ///
    /// Future [`Fs::read_dir`] or [`Fs::read_dir_plus`] calls may provide this
    /// value from [`ReadDirReq::cookie`] or [`ReadDirPlus::cookie`] to resume
    /// reading starting with the entry following this one.
    ///
    /// Note that official FUSE documentation names this field `offset`, which
    /// can be misleading since the value can be arbitrary and need not be
    /// ordered across sequential entries. They only need to be usable by the
    /// filesystem to locate the initial entry on a new directory read.
    ///
    /// [Fs::read_dir]: crate::fs::Fs::read_dir
    /// [Fs::read_dir_plus]: crate::fs::Fs::read_dir_plus
    /// [ReadDirPlus::cookie]: super::ReadDirPlus::cookie
    pub cookie: u64,

    pub name: T,
}

/// `FUSE_DIRENT_ALIGN`: every entry is padded to a multiple of this.
pub(crate) const DIRENT_ALIGN: usize = std::mem::size_of::<u64>();
pub(crate) const DIRENT_PADDING: [u8; DIRENT_ALIGN] = [0; DIRENT_ALIGN];

#[repr(C)]
pub struct RawDirEntry {
    pub ino: u64,
    pub cookie: u64,
    pub namelen: u32,
    pub kind: u32,
}

impl ReadDirReq {
    pub(crate) fn new(req: Req, readdir: ReadDir) -> Self {
        Self { req, readdir }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.readdir.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.readdir.file_handle()
    }

    pub fn cookie(&self) -> u64 {
        self.readdir.offset()
    }

    pub fn capacity(&self) -> usize {
        self.readdir.len()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.readdir.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.readdir.open_flags()
    }

    pub fn new_buffer(&self) -> DirEntryBuf {
        let cap = self.capacity();
        DirEntryBuf {
            max_len: cap,
            cookie: self.cookie(),
            count: 0,
            buf: self.buffer_pool().checkout_with_capacity(cap),
        }
    }

    pub fn collect_entries<I, T>(&self, iter: I) -> Result<DirEntryBuf>
    where
        I: IntoIterator<Item = DirEntry<T>>,
        T: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend(iter)?;
        Ok(buf)
    }

    pub async fn collect_entry_stream<S, T>(&self, stream: S) -> Result<DirEntryBuf>
    where
        S: Stream<Item = DirEntry<T>>,
        T: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend_stream(stream).await?;
        Ok(buf)
    }
}

impl std::ops::Deref for ReadDirReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl DirEntryBuf {
    pub fn last_cookie(&self) -> u64 {
        self.cookie
    }

    pub fn num_entries(&self) -> usize {
        self.count
    }

    pub fn remaining_capacity(&self) -> usize {
        self.max_len - self.buf.len()
    }

    pub fn has_capacity_for<T>(&self, name: T) -> bool
    where
        T: AsRef<OsStr>,
    {
        DirEntry::entry_size_for(&name) <= self.remaining_capacity()
    }

    pub fn push<T>(&mut self, entry: &DirEntry<T>) -> Result<bool>
    where
        T: AsRef<OsStr>,
    {
        let name = entry.name.as_ref().as_bytes();
        let namelen = u32::try_from(name.len()).map_err(|_| Error::E2BIG)?;

        let entry_len = (namelen as usize) + std::mem::size_of::<RawDirEntry>();
        let padded_entry_len = entry_len.next_multiple_of(DIRENT_ALIGN);
        let padding_len = padded_entry_len - entry_len;
        if padded_entry_len > self.remaining_capacity() {
            return Ok(false);
        }

        let raw = RawDirEntry {
            ino: entry.ino.as_raw(),
            cookie: entry.cookie,
            namelen,
            kind: u32::from(SFlag::from(entry.kind).bits()) >> 12,
        };
        let raw = unsafe {
            std::slice::from_raw_parts(
                &raw as *const RawDirEntry as *const u8,
                std::mem::size_of::<RawDirEntry>(),
            )
        };

        self.buf.extend_from_slice(raw);
        self.buf.extend_from_slice(name);
        self.buf.extend_from_slice(&DIRENT_PADDING[..padding_len]);

        self.count += 1;
        self.cookie = entry.cookie;

        Ok(true)
    }

    pub fn extend<I, T>(&mut self, iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = DirEntry<T>>,
        T: AsRef<OsStr>,
    {
        let start = self.count;
        for entry in iter {
            if !self.push(&entry)? {
                break;
            }
        }
        Ok(self.count - start)
    }

    pub async fn extend_stream<S, T>(&mut self, stream: S) -> Result<usize>
    where
        S: Stream<Item = DirEntry<T>>,
        T: AsRef<OsStr>,
    {
        let mut stream = std::pin::pin!(stream);
        let start = self.count;
        while let Some(entry) = stream.next().await
            && self.push(&entry)?
        {}
        Ok(self.count - start)
    }

    pub(crate) fn into_data(self) -> Data<Buf> {
        Data::new(self.buf)
    }
}

impl<T: AsRef<OsStr>> DirEntry<T> {
    pub fn entry_size_for(name: &T) -> usize {
        name.as_ref()
            .as_bytes()
            .len()
            .next_multiple_of(DIRENT_ALIGN)
            + std::mem::size_of::<RawDirEntry>()
    }

    pub fn entry_size(&self) -> usize {
        Self::entry_size_for(&self.name)
    }
}
