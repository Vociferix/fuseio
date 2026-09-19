mod buf;
mod pool;

pub use buf::Buf;
pub use pool::BufPool;

pub(crate) const ALIGN: usize = std::mem::align_of::<crossbeam_utils::CachePadded<u64>>();

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

pub struct IoBufferWithNul<B, V> {
    buf: IoBuffer<B, V>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BufferPlaceholder {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EitherBuf<L, R>(either::Either<L, R>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EitherIntoIoBuf<L, R>(either::Either<L, R>);

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

impl<B, V> IoBufferWithNul<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    fn new(buf: IoBuffer<B, V>) -> Self {
        Self { buf }
    }
}

pub trait IntoIoBuf: sealed::Sealed + Sized {
    #[doc(hidden)]
    type Buffer: compio::buf::IoBuf;

    #[doc(hidden)]
    type VecBuffer: compio::buf::IoVectoredBuf;

    #[doc(hidden)]
    fn into_io_buf(self) -> IoBuffer<Self::Buffer, Self::VecBuffer>;

    #[doc(hidden)]
    fn into_io_buf_with_nul(self) -> IoBufferWithNul<Self::Buffer, Self::VecBuffer> {
        IoBufferWithNul::new(self.into_io_buf())
    }

    fn left_buf<R>(self) -> EitherIntoIoBuf<Self, R> {
        EitherIntoIoBuf(either::Left(self))
    }

    fn right_buf<L>(self) -> EitherIntoIoBuf<L, Self> {
        EitherIntoIoBuf(either::Right(self))
    }
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

impl<B, V> compio::buf::IoVectoredBuf for IoBuffer<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
        match self {
            Self::Buf(buf) => either::Left(std::iter::once(buf.as_init())),
            Self::VecBuf(buf) => either::Right(buf.iter_slice()),
        }
    }

    fn total_len(&self) -> usize {
        match self {
            Self::Buf(buf) => buf.buf_len(),
            Self::VecBuf(buf) => buf.total_len(),
        }
    }
}

impl<B, V> sealed::Sealed for IoBuffer<B, V> {}

impl<B, V> IntoIoBuf for IoBuffer<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    type Buffer = B;
    type VecBuffer = V;

    fn into_io_buf(self) -> Self {
        self
    }

    fn into_io_buf_with_nul(self) -> IoBufferWithNul<Self::Buffer, Self::VecBuffer> {
        IoBufferWithNul::new(self)
    }
}

const NUL: &[u8] = &[0];

impl<B, V> compio::buf::IoVectoredBuf for IoBufferWithNul<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
        match &self.buf {
            IoBuffer::Buf(buf) => either::Left([buf.as_init(), NUL].into_iter()),
            IoBuffer::VecBuf(buf) => either::Right(buf.iter_slice().chain(std::iter::once(NUL))),
        }
    }

    fn total_len(&self) -> usize {
        self.buf.total_len() + 1
    }
}

impl<B, V> sealed::Sealed for IoBufferWithNul<B, V> {}

impl<B, V> IntoIoBuf for IoBufferWithNul<B, V>
where
    B: compio::buf::IoBuf,
    V: compio::buf::IoVectoredBuf,
{
    type Buffer = BufferPlaceholder;
    type VecBuffer = Self;

    fn into_io_buf(self) -> IoBuffer<Self::Buffer, Self> {
        IoBuffer::VecBuf(self)
    }
}

impl<L, R> compio::buf::IoBuf for EitherBuf<L, R>
where
    L: compio::buf::IoBuf,
    R: compio::buf::IoBuf,
{
    fn as_init(&self) -> &[u8] {
        match &self.0 {
            either::Left(l) => l.as_init(),
            either::Right(r) => r.as_init(),
        }
    }

    fn buf_len(&self) -> usize {
        match &self.0 {
            either::Left(l) => l.buf_len(),
            either::Right(r) => r.buf_len(),
        }
    }

    fn buf_ptr(&self) -> *const u8 {
        match &self.0 {
            either::Left(l) => l.buf_ptr(),
            either::Right(r) => r.buf_ptr(),
        }
    }

    fn is_empty(&self) -> bool {
        match &self.0 {
            either::Left(l) => l.is_empty(),
            either::Right(r) => r.is_empty(),
        }
    }
}

impl<L, R> compio::buf::IoVectoredBuf for EitherBuf<L, R>
where
    L: compio::buf::IoVectoredBuf,
    R: compio::buf::IoVectoredBuf,
{
    fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
        match &self.0 {
            either::Left(l) => either::Left(l.iter_slice()),
            either::Right(r) => either::Right(r.iter_slice()),
        }
    }

    fn total_len(&self) -> usize {
        match &self.0 {
            either::Left(l) => l.total_len(),
            either::Right(r) => r.total_len(),
        }
    }
}

impl<L, R> sealed::Sealed for EitherIntoIoBuf<L, R> {}

impl<L, R> IntoIoBuf for EitherIntoIoBuf<L, R>
where
    L: IntoIoBuf,
    R: IntoIoBuf,
{
    type Buffer = EitherBuf<L::Buffer, R::Buffer>;
    type VecBuffer = EitherBuf<L::VecBuffer, R::VecBuffer>;

    fn into_io_buf(self) -> IoBuffer<Self::Buffer, Self::VecBuffer> {
        match self.0 {
            either::Left(l) => match l.into_io_buf() {
                IoBuffer::Buf(buf) => IoBuffer::Buf(EitherBuf(either::Left(buf))),
                IoBuffer::VecBuf(buf) => IoBuffer::VecBuf(EitherBuf(either::Left(buf))),
            },
            either::Right(r) => match r.into_io_buf() {
                IoBuffer::Buf(buf) => IoBuffer::Buf(EitherBuf(either::Right(buf))),
                IoBuffer::VecBuf(buf) => IoBuffer::VecBuf(EitherBuf(either::Right(buf))),
            },
        }
    }
}
