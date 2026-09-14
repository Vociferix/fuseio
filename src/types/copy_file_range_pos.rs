use super::{FileHandle, Ino};

#[derive(Debug)]
pub struct CopyFileRangePos {
    pub(crate) ino: Ino,
    pub(crate) fh: FileHandle,
    pub(crate) offset: u64,
}

impl CopyFileRangePos {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }
}
