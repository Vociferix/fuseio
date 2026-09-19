use super::Ino;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct ForgetOne {
    nodeid: u64,
    nlookup: u64,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForgetIno {
    inner: ForgetOne,
}

impl ForgetIno {
    pub(crate) fn new(nodeid: Ino, nlookup: u64) -> Self {
        Self {
            inner: ForgetOne {
                nodeid: nodeid.as_raw(),
                nlookup,
            },
        }
    }

    pub fn ino(&self) -> Ino {
        unsafe { Ino::from_raw_unchecked(self.inner.nodeid) }
    }

    pub fn num_lookups(&self) -> u64 {
        self.inner.nlookup
    }
}

impl From<ForgetIno> for Ino {
    fn from(forget: ForgetIno) -> Ino {
        forget.ino()
    }
}
