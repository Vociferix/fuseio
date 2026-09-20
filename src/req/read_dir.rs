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
    buf: Buf,
}

#[derive(Debug)]
pub struct DirEntry<T> {
    pub ino: Ino,
    pub kind: InodeKind,
    pub next: u64,
    pub name: T,
}

/// `FUSE_DIRENT_ALIGN`: every entry is padded to a multiple of this.
pub(crate) const DIRENT_ALIGN: usize = std::mem::size_of::<u64>();
pub(crate) const DIRENT_PADDING: [u8; DIRENT_ALIGN] = [0; DIRENT_ALIGN];

#[repr(C)]
pub struct RawDirEntry {
    pub ino: u64,
    pub next: u64,
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

    pub fn new_buffer(&self) -> DirEntryBuf {
        let len = self.len();
        DirEntryBuf {
            max_len: len,
            buf: self.buffer_pool().checkout_with_capacity(len),
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
    pub fn push<T>(&mut self, entry: DirEntry<T>) -> Result<()>
    where
        T: AsRef<OsStr>,
    {
        let name = entry.name.as_ref().as_bytes();

        let Some(entry_len) = name
            .len()
            .checked_add(std::mem::size_of::<RawDirEntry>())
            .filter(|len| *len <= self.max_len)
        else {
            return Err(Error::ERANGE);
        };

        let Some(total_len) = entry_len
            .checked_next_multiple_of(DIRENT_ALIGN)
            .and_then(|padded| self.buf.len().checked_add(padded))
        else {
            return Err(Error::ERANGE);
        };
        if total_len > self.max_len {
            return Err(Error::ERANGE);
        }

        let Ok(namelen) = u32::try_from(name.len()) else {
            return Err(Error::E2BIG);
        };

        let raw = RawDirEntry {
            ino: entry.ino.as_raw(),
            next: entry.next,
            namelen,
            // The wire format carries the DT_* value, i.e. the file type bits of
            // the mode shifted down.
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
        // Pad the entry out so the next one starts aligned.
        self.buf.extend_from_slice(
            &DIRENT_PADDING[..entry_len.next_multiple_of(DIRENT_ALIGN) - entry_len],
        );

        Ok(())
    }

    pub fn extend<I, T>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = DirEntry<T>>,
        T: AsRef<OsStr>,
    {
        for entry in iter {
            self.push(entry)?;
        }
        Ok(())
    }

    pub async fn extend_stream<S, T>(&mut self, stream: S) -> Result<()>
    where
        S: Stream<Item = DirEntry<T>>,
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
