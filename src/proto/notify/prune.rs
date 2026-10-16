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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode, u32_at, u64_at};

    const COUNT: usize = HDR;
    const INOS: usize = HDR + 16;

    fn prune(inos: &[u64]) -> Prune {
        let pool = BufPool::new();
        let mut prune = Prune::with_capacity(&pool, inos.len());

        for &ino in inos {
            prune.push(Ino::from_raw(ino).unwrap());
        }

        prune
    }

    // `fuse_notify_prune_out { count, padding, spare }`, code 9, then `count`
    // inode numbers. The kernel checks the length exactly, so the padding and
    // the spare word have to be there.
    #[test]
    fn the_inodes_follow_a_sixteen_byte_description() {
        let bytes = encode(prune(&[2, 3, 5]));

        assert_notify_header(&bytes, 9);
        assert_eq!(u32_at(&bytes, COUNT), 3);
        assert_eq!(u32_at(&bytes, COUNT + 4), 0, "the padding must be zero");
        assert_eq!(u64_at(&bytes, COUNT + 8), 0, "the spare word must be zero");
        assert_eq!(bytes.len(), INOS + 3 * 8, "the kernel checks this exactly");

        for (i, expected) in [2u64, 3, 5].into_iter().enumerate() {
            assert_eq!(u64_at(&bytes, INOS + i * 8), expected);
        }
    }

    #[test]
    fn pruning_nothing_is_a_description_with_no_inodes() {
        let bytes = encode(prune(&[]));

        assert_notify_header(&bytes, 9);
        assert_eq!(u32_at(&bytes, COUNT), 0);
        assert_eq!(bytes.len(), INOS);
    }

    #[test]
    fn the_count_follows_what_was_pushed() {
        let pool = BufPool::new();
        let mut p = Prune::with_capacity(&pool, 0);
        p.extend_from_slice(&[Ino::from_raw(7).unwrap(), Ino::from_raw(8).unwrap()]);

        assert_eq!(p.len(), 2);
        assert!(!p.is_empty());

        let bytes = encode(p);

        assert_eq!(u32_at(&bytes, COUNT), 2);
        assert_eq!(bytes.len(), INOS + 16);
    }
}
