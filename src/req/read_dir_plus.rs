use super::{
    DirEntry, Req,
    read_dir::{DIRENT_ALIGN, DIRENT_PADDING, RawDirEntry},
};
use crate::buf::Buf;
use crate::proto::request::ReadDirPlus;
use crate::proto::response::{Data, Entry};
use crate::types::{FileHandle, Ino, InodeKind, LockOwner, OFlag, SFlag};
use crate::{Error, Result};

use futures_util::{Stream, StreamExt};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct ReadDirPlusReq {
    req: Req,
    readdir: ReadDirPlus,
}

#[derive(Debug)]
pub struct DirEntryPlusBuf {
    max_len: usize,
    cookie: u64,
    count: usize,
    buf: Buf,
}

#[derive(Debug)]
pub struct DirEntryPlus<T> {
    pub entry: Entry,
    pub dirent: DirEntry<T>,
}

#[repr(C)]
pub struct RawDirEntryPlus {
    entry: Entry,
    dirent: RawDirEntry,
}

impl ReadDirPlusReq {
    pub(crate) fn new(req: Req, readdir: ReadDirPlus) -> Self {
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

    pub fn new_buffer(&self) -> DirEntryPlusBuf {
        let cap = self.capacity();
        DirEntryPlusBuf {
            max_len: cap,
            cookie: self.cookie(),
            count: 0,
            buf: self.buffer_pool().checkout_with_capacity(cap),
        }
    }

    pub fn collect_entries<I, T>(&self, iter: I) -> Result<DirEntryPlusBuf>
    where
        I: IntoIterator<Item = DirEntryPlus<T>>,
        T: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend(iter)?;
        Ok(buf)
    }

    pub async fn collect_entry_stream<S, T>(&self, stream: S) -> Result<DirEntryPlusBuf>
    where
        S: Stream<Item = DirEntryPlus<T>>,
        T: AsRef<OsStr>,
    {
        let mut buf = self.new_buffer();
        buf.extend_stream(stream).await?;
        Ok(buf)
    }
}

impl std::ops::Deref for ReadDirPlusReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl DirEntryPlusBuf {
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
        DirEntryPlus::entry_size_for(&name) <= self.remaining_capacity()
    }

    pub fn push<T>(&mut self, entry: &DirEntryPlus<T>) -> Result<bool>
    where
        T: AsRef<OsStr>,
    {
        let name = entry.dirent.name.as_ref().as_bytes();
        let namelen = u32::try_from(name.len()).map_err(|_| Error::E2BIG)?;

        let entry_len = (namelen as usize) + std::mem::size_of::<RawDirEntryPlus>();
        let padded_entry_len = entry_len.next_multiple_of(DIRENT_ALIGN);
        let padding_len = padded_entry_len - entry_len;
        if padded_entry_len > self.remaining_capacity() {
            return Ok(false);
        }

        let dirent = RawDirEntry {
            ino: entry.dirent.ino.as_raw(),
            cookie: entry.dirent.cookie,
            namelen,
            kind: u32::from(SFlag::from(entry.dirent.kind).bits()) >> 12,
        };
        let raw = RawDirEntryPlus {
            entry: unsafe { std::ptr::read(&entry.entry) },
            dirent,
        };
        let raw = unsafe {
            std::slice::from_raw_parts(
                &raw as *const RawDirEntryPlus as *const u8,
                std::mem::size_of::<RawDirEntryPlus>(),
            )
        };

        self.buf.extend_from_slice(raw);
        self.buf.extend_from_slice(name);
        self.buf.extend_from_slice(&DIRENT_PADDING[..padding_len]);

        self.count += 1;
        self.cookie = entry.dirent.cookie;

        Ok(true)
    }

    pub fn extend<I, T>(&mut self, iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = DirEntryPlus<T>>,
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
        S: Stream<Item = DirEntryPlus<T>>,
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

impl<T: AsRef<OsStr>> DirEntryPlus<T> {
    pub fn entry_size_for(name: &T) -> usize {
        name.as_ref()
            .as_bytes()
            .len()
            .next_multiple_of(DIRENT_ALIGN)
            + std::mem::size_of::<RawDirEntryPlus>()
    }

    pub fn entry_size(&self) -> usize {
        Self::entry_size_for(&self.dirent.name)
    }
}

impl<T> DirEntry<T> {
    pub fn plus(self, entry: Entry) -> DirEntryPlus<T> {
        DirEntryPlus {
            entry,
            dirent: self,
        }
    }
}
