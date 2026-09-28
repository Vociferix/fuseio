use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::{Buf, BufPool};
use crate::types::Ino;

#[derive(Debug)]
pub struct Prune {
    inos: Buf,
}

#[repr(C)]
struct Out {
    hdr: RawHeader,
    count: u32,
    _unused: [u32; 3],
}

const OUT_LEN: usize = std::mem::size_of::<Out>();

impl Prune {
    pub fn with_capacity(pool: &BufPool, capacity: usize) -> Self {
        let cap = OUT_LEN + (capacity * std::mem::size_of::<Ino>());
        let mut buf = pool.checkout_with_capacity(cap);
        buf.resize(OUT_LEN, 0);

        Self { inos: buf }
    }

    pub fn push(&mut self, ino: Ino) {
        let bytes = unsafe {
            std::slice::from_raw_parts(&ino as *const Ino as *const u8, std::mem::size_of::<Ino>())
        };
        self.inos.extend_from_slice(bytes);
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inos.reserve(additional * std::mem::size_of::<Ino>());
    }

    pub fn as_slice(&self) -> &[Ino] {
        let ptr = unsafe { self.inos.as_ptr().add(OUT_LEN).cast() };
        let len = self.len();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    pub fn is_empty(&self) -> bool {
        self.inos.len() == OUT_LEN
    }

    pub fn len(&self) -> usize {
        (self.inos.len() - OUT_LEN) / std::mem::size_of::<Ino>()
    }
}

impl EncodeNotify for Prune {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        let mut buf = self.inos;
        let Ok(len) = u32::try_from(buf.len()) else {
            return Err(crate::Error::EIO);
        };

        let count = ((buf.len() - OUT_LEN) / std::mem::size_of::<Ino>()) as u32;

        {
            let out = unsafe { &mut *(buf.as_mut_ptr() as *mut Out) };
            out.hdr.len = len;
            out.hdr.code = NotifyCode::PRUNE;
            out.count = count;
        }

        Ok(buf)
    }
}
