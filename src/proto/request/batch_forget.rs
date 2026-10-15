use super::{Cfg, Ino, RawHeader};
use crate::types::ForgetIno;
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

#[derive(Debug)]
pub struct BatchForget {
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    count: u32,
    _unused: u32,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct ForgetOne {
    nodeid: u64,
    nlookup: u64,
}

const _: () = {
    assert!(std::mem::size_of::<ForgetOne>() == std::mem::size_of::<ForgetIno>());
    assert!(std::mem::align_of::<ForgetOne>() == std::mem::align_of::<ForgetIno>());
};

impl BatchForget {
    pub fn inos(&self) -> &[ForgetIno] {
        let len = self.buf.len() / std::mem::size_of::<ForgetIno>();
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr().cast(), len) }
    }
}

impl BatchForget {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, forgets)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let count = raw.count as usize;

        let Some((forgets, _)) = forgets.split_slice::<ForgetOne>(count) else {
            return Err(Error::EPROTO);
        };

        if forgets.iter().any(|forget| forget.nodeid == 0) {
            return Err(Error::EINVAL);
        }

        let Some(buf) = buf.try_into_view(
            std::mem::size_of::<Raw>()
                ..(std::mem::size_of::<Raw>() + count * std::mem::size_of::<ForgetOne>()),
        ) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { buf })
    }
}
