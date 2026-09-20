use super::{Cfg, HDR_LEN, Ino};
use crate::types::ForgetIno;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct BatchForget {
    buf: Buf,
    count: usize,
}

#[repr(C)]
struct Raw {
    count: u32,
    _unused: u32,
}

#[repr(C)]
struct ForgetOne {
    nodeid: u64,
    nlookup: u64,
}

const DATA_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

impl BatchForget {
    pub fn inos(&self) -> &[ForgetIno] {
        unsafe {
            std::slice::from_raw_parts(
                self.buf.as_ptr().add(DATA_OFFSET) as *const ForgetIno,
                self.count,
            )
        }
    }
}

impl BatchForget {
    pub(super) fn decode(mut buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < DATA_OFFSET {
            return Err(Error::EPROTO);
        }

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };
        let count = raw.count as usize;

        if buf.len() < DATA_OFFSET + (count * std::mem::size_of::<ForgetOne>()) {
            return Err(Error::EPROTO);
        }

        {
            let forgets = unsafe {
                std::slice::from_raw_parts(buf.as_ptr().add(DATA_OFFSET) as *const ForgetOne, count)
            };

            for forget in forgets {
                if forget.nodeid == 0 {
                    return Err(Error::EINVAL);
                }
            }
        }

        buf.truncate(DATA_OFFSET + (count * std::mem::size_of::<ForgetOne>()));

        Ok(Self { buf, count })
    }
}
