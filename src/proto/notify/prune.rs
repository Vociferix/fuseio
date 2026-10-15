use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::{Buf, BufPool};
use crate::types::Ino;

#[derive(Debug)]
pub struct Prune {
    inos: Buf,
}

#[derive(Clone, Copy, bytemuck::NoUninit, bytemuck::AnyBitPattern)]
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
        self.inos.extend_from_slice(bytemuck::bytes_of(&ino));
    }

    pub fn extend_from_slice(&mut self, inos: &[Ino]) {
        self.inos.extend_from_slice(bytemuck::cast_slice(inos));
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inos.reserve(additional * std::mem::size_of::<Ino>());
    }

    pub fn as_slice(&self) -> &[Ino] {
        // SAFETY: `Prune` guarantees `self.inos[OUT_LEN..]` contains a valid `Ino` slice.
        // The first `OUT_LEN` bytes are always reserved on construction (see `with_capacity`)
        // and the `push` and `extend_from_slice` methods are the only ways to append data and
        // only support appending `Ino`s. Also note that `Ino` transparently wraps `NonZeroU64`.
        //
        // Unfortunately, because `Ino` has invalid bit patterns, we can't use `bytemuck` to
        // cast from the bytes to `[Ino]`, so we have to manually perform the cast, which means
        // `unsafe { }`. But as explained in the first paragraph, we know the bytes do not contain
        // such invalid bit patterns, so this cast is safe.
        unsafe { std::slice::from_raw_parts(self.inos.as_ptr().add(OUT_LEN).cast(), self.len()) }
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

        let Some((out, _)) = buf.as_slice_mut().split_first_mut::<Out>() else {
            return Err(crate::Error::EIO);
        };
        out.hdr.len = len;
        out.hdr.code = NotifyCode::PRUNE;
        out.count = count;

        Ok(buf)
    }
}
