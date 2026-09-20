use super::{DirEntry, RawDirEntry, Req};
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
    buf: Buf,
}

#[repr(C)]
struct RawDirEntryPlus {
    entry: Entry,
    dirent: RawDirEntry,
}

#[derive(Debug)]
pub struct DirEntryPlus<T> {
    pub entry: Entry,
    pub dirent: DirEntry<T>,
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

    pub fn offset(&self) -> u64 {
        self.readdir.offset()
    }

    pub fn len(&self) -> usize {
        self.readdir.len()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.readdir.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.readdir.open_flags()
    }

    pub fn new_buffer(&self) -> DirEntryPlusBuf {
        let len = self.len();
        DirEntryPlusBuf {
            max_len: len,
            buf: self.buffer_pool().checkout_with_capacity(len),
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
    pub fn push<T>(&mut self, entry: DirEntryPlus<T>) -> Result<()>
    where
        T: AsRef<OsStr>,
    {
        let name = entry.dirent.name.as_ref().as_bytes();

        let Some(total_len) = self
            .buf
            .len()
            .checked_add(name.len())
            .and_then(|len| len.checked_add(std::mem::size_of::<RawDirEntryPlus>()))
        else {
            return Err(Error::ERANGE);
        };
        if total_len > self.max_len {
            return Err(Error::ERANGE);
        }

        let Ok(namelen) = u32::try_from(name.len()) else {
            return Err(Error::E2BIG);
        };

        let dirent = RawDirEntry {
            ino: entry.dirent.ino.as_raw(),
            next: entry.dirent.next,
            namelen,
            kind: SFlag::from(entry.dirent.kind).bits(),
        };
        let raw = RawDirEntryPlus {
            entry: entry.entry,
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

        Ok(())
    }

    pub fn extend<I, T>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = DirEntryPlus<T>>,
        T: AsRef<OsStr>,
    {
        for entry in iter {
            self.push(entry)?;
        }
        Ok(())
    }

    pub async fn extend_stream<S, T>(&mut self, stream: S) -> Result<()>
    where
        S: Stream<Item = DirEntryPlus<T>>,
        T: AsRef<OsStr>,
    {
        let mut stream = std::pin::pin!(stream);

        while let Some(entry) = stream.next().await {
            self.push(entry)?;
        }
        Ok(())
    }

    pub(crate) fn into_data(self) -> Data<Buf> {
        Data::new(self.buf)
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
