mod buf;
mod pool;

pub use buf::Buf;
pub use pool::BufPool;

const ALIGN: usize = std::mem::align_of::<crossbeam_utils::CachePadded<u64>>();

mod sealed {
    pub trait Sealed {}

    impl<B: compio::buf::IoBuf> Sealed for B {}

    impl<V: compio::buf::IoVectoredBuf> Sealed for super::Vectored<V> {}
}

pub struct Vectored<V>(pub V);

pub enum IoBuffer<B, V> {
    Buf(B),
    VecBuf(V),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BufferPlaceholder {}

impl<B, V> IoBuffer<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    pub fn total_len(&self) -> usize {
        match self {
            Self::Buf(buf) => buf.buf_len(),
            Self::VecBuf(bufs) => bufs.total_len(),
        }
    }

    pub fn into_vectored(self) -> impl compio::buf::IoVectoredBuf {
        enum VecBuf<B, V> {
            Buf(B),
            VecBuf(V),
        }

        impl<B, V> compio::buf::IoVectoredBuf for VecBuf<B, V>
        where
            B: compio::buf::IoBuf,
            V: compio::buf::IoVectoredBuf,
        {
            fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
                match self {
                    Self::Buf(buf) => either::Left(std::iter::once(buf.as_init())),
                    Self::VecBuf(bufs) => either::Right(bufs.iter_slice()),
                }
            }

            fn total_len(&self) -> usize {
                match self {
                    Self::Buf(buf) => buf.buf_len(),
                    Self::VecBuf(bufs) => bufs.total_len(),
                }
            }
        }

        match self {
            Self::Buf(buf) => VecBuf::Buf(buf),
            Self::VecBuf(bufs) => VecBuf::VecBuf(bufs),
        }
    }
}

pub trait IntoIoBuf: sealed::Sealed {
    #[doc(hidden)]
    type Buffer: compio::buf::IoBuf;

    #[doc(hidden)]
    type VecBuffer: compio::buf::IoVectoredBuf;

    #[doc(hidden)]
    fn into_io_buf(self) -> IoBuffer<Self::Buffer, Self::VecBuffer>;
}

impl compio::buf::IoBuf for BufferPlaceholder {
    fn as_init(&self) -> &[u8] {
        unsafe { std::hint::unreachable_unchecked() }
    }
}

impl compio::buf::IoVectoredBuf for BufferPlaceholder {
    fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
        enum Iter<'a> {
            __Uninhabited(BufferPlaceholder, std::marker::PhantomData<&'a [u8]>),
        }

        impl<'a> Iterator for Iter<'a> {
            type Item = &'a [u8];

            fn next(&mut self) -> Option<Self::Item> {
                unsafe { std::hint::unreachable_unchecked() }
            }
        }

        fn iter<'a>() -> Iter<'a> {
            unsafe { std::hint::unreachable_unchecked() }
        }

        iter()
    }
}

impl<B: compio::buf::IoBuf> IntoIoBuf for B {
    type Buffer = Self;
    type VecBuffer = BufferPlaceholder;

    fn into_io_buf(self) -> IoBuffer<Self, BufferPlaceholder> {
        IoBuffer::Buf(self)
    }
}

impl<V: compio::buf::IoVectoredBuf> IntoIoBuf for Vectored<V> {
    type Buffer = BufferPlaceholder;
    type VecBuffer = V;

    fn into_io_buf(self) -> IoBuffer<BufferPlaceholder, V> {
        IoBuffer::VecBuf(self.0)
    }
}
