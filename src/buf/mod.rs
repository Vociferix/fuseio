#[allow(clippy::module_inception)]
mod buf;
mod pool;

pub use buf::Buf;
pub use pool::BufPool;

pub(crate) use buf::BufView;

pub(crate) const ALIGN: usize = std::mem::align_of::<u64>();

mod sealed {
    pub trait Sealed {}

    impl<B: compio::buf::IoBuf> Sealed for B {}

    impl<V: compio::buf::IoVectoredBuf> Sealed for super::Vectored<V> {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Vectored<V>(pub V);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Pod<T>(pub T);

pub(crate) enum IoBuffer<B, V> {
    Buf(B),
    VecBuf(V),
}

pub(crate) struct IoBufferWithNul<B, V> {
    buf: IoBuffer<B, V>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoData {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EitherBuf<L, R>(either::Either<L, R>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EitherIntoIoBuf<L, R>(either::Either<L, R>);

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

    /// The length without the terminator, which is what the kernel's `namelen`
    /// fields count.
    pub(crate) fn len_without_nul(&self) -> usize {
        self.buf.total_len()
    }
}

pub trait IntoIoBuf: sealed::Sealed + Sized {
    #[doc(hidden)]
    type Buffer: compio::buf::IoBuf;

    #[doc(hidden)]
    type VecBuffer: compio::buf::IoVectoredBuf;

    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn into_io_buf(self) -> IoBuffer<Self::Buffer, Self::VecBuffer>;

    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn into_io_buf_with_nul(self) -> IoBufferWithNul<Self::Buffer, Self::VecBuffer> {
        IoBufferWithNul::new(self.into_io_buf())
    }

    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn left_buf<R>(self) -> EitherIntoIoBuf<Self, R> {
        EitherIntoIoBuf(either::Left(self))
    }

    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn right_buf<L>(self) -> EitherIntoIoBuf<L, Self> {
        EitherIntoIoBuf(either::Right(self))
    }
}

impl compio::buf::IoBuf for NoData {
    fn as_init(&self) -> &[u8] {
        b""
    }
}

impl compio::buf::IoVectoredBuf for NoData {
    fn iter_slice(&self) -> impl Iterator<Item = &[u8]> {
        enum Iter<'a> {
            __Uninhabited(NoData, std::marker::PhantomData<&'a [u8]>),
        }

        impl<'a> Iterator for Iter<'a> {
            type Item = &'a [u8];

            fn next(&mut self) -> Option<Self::Item> {
                None
            }
        }

        fn iter<'a>() -> Iter<'a> {
            unreachable!()
        }

        iter()
    }
}

impl<B: compio::buf::IoBuf> IntoIoBuf for B {
    type Buffer = Self;
    type VecBuffer = NoData;

    #[allow(private_interfaces)]
    fn into_io_buf(self) -> IoBuffer<Self, NoData> {
        IoBuffer::Buf(self)
    }
}

impl<V: compio::buf::IoVectoredBuf> IntoIoBuf for Vectored<V> {
    type Buffer = NoData;
    type VecBuffer = V;

    #[allow(private_interfaces)]
    fn into_io_buf(self) -> IoBuffer<NoData, V> {
        IoBuffer::VecBuf(self.0)
    }
}

impl<T: bytemuck::NoUninit> compio::buf::IoBuf for Pod<T> {
    fn as_init(&self) -> &[u8] {
        bytemuck::bytes_of(&self.0)
    }

    fn buf_len(&self) -> usize {
        std::mem::size_of::<T>()
    }

    fn buf_ptr(&self) -> *const u8 {
        &self.0 as *const T as *const u8
    }

    fn is_empty(&self) -> bool {
        std::mem::size_of::<T>() == 0
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
    type Buffer = NoData;
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

#[cfg(test)]
mod tests {
    use super::*;

    use compio::buf::{IoBuf, IoVectoredBuf};

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::NoUninit)]
    struct Header {
        len: u32,
        opcode: u32,
        unique: u64,
    }

    fn gathered(bufs: &impl IoVectoredBuf) -> Vec<u8> {
        bufs.iter_slice().flatten().copied().collect()
    }

    // `Pod` exists so a `#[repr(C)]` reply can go on the wire without a
    // hand-written `IoBuf`. What it writes has to be the struct's own bytes, in
    // order, with nothing added.
    #[test]
    fn a_pod_is_exactly_its_own_bytes() {
        let pod = Pod(Header {
            len: 0x1122_3344,
            opcode: 26,
            unique: 7,
        });

        let mut expected = Vec::new();
        expected.extend_from_slice(&0x1122_3344u32.to_ne_bytes());
        expected.extend_from_slice(&26u32.to_ne_bytes());
        expected.extend_from_slice(&7u64.to_ne_bytes());

        assert_eq!(pod.as_init(), expected.as_slice());
        assert_eq!(pod.buf_len(), std::mem::size_of::<Header>());
        assert!(!pod.is_empty());
    }

    // The kernel's `namelen` fields count the name without its terminator, so
    // the two lengths must differ by exactly the one byte.
    #[test]
    fn a_nul_terminated_buffer_adds_one_byte_that_the_name_length_excludes() {
        let with_nul = b"hello.txt".to_vec().into_io_buf_with_nul();

        assert_eq!(with_nul.len_without_nul(), 9);
        assert_eq!(with_nul.total_len(), 10);
        assert_eq!(gathered(&with_nul), b"hello.txt\0");
    }

    #[test]
    fn an_empty_name_is_still_terminated() {
        let with_nul = Vec::new().into_io_buf_with_nul();

        assert_eq!(with_nul.len_without_nul(), 0);
        assert_eq!(with_nul.total_len(), 1);
        assert_eq!(gathered(&with_nul), b"\0");
    }

    // A name assembled from several pieces is terminated once, at the end,
    // rather than after each piece.
    #[test]
    fn a_vectored_name_is_terminated_once() {
        let pieces = Vectored(vec![b"dir".to_vec(), b"/".to_vec(), b"file".to_vec()]);
        let with_nul = pieces.into_io_buf_with_nul();

        assert_eq!(with_nul.len_without_nul(), 8);
        assert_eq!(with_nul.total_len(), 9);
        assert_eq!(gathered(&with_nul), b"dir/file\0");
    }

    #[test]
    fn a_vectored_buffer_reports_the_sum_of_its_pieces() {
        let pieces = Vectored(vec![b"ab".to_vec(), b"cde".to_vec()]);

        // The `Buf` variant is unreachable here: `Vectored`'s single-buffer
        // half is `NoData`, which is uninhabited, so the type system already
        // rules out a vectored buffer collapsing into a flat one.
        let IoBuffer::VecBuf(bufs) = pieces.into_io_buf();

        assert_eq!(bufs.total_len(), 5);
        assert_eq!(gathered(&bufs), b"abcde");
    }

    #[test]
    fn a_flat_buffer_reports_its_own_length() {
        // Unreachable the other way round for the same reason: a plain
        // `IoBuf`'s vectored half is `NoData`.
        let IoBuffer::Buf(buf) = b"flat".to_vec().into_io_buf();

        assert_eq!(buf.buf_len(), 4);
    }

    #[test]
    fn either_keeps_whichever_side_it_was_given() {
        let left = b"left".to_vec().left_buf::<Vec<u8>>();
        let right = b"right".to_vec().right_buf::<Vec<u8>>();

        assert_eq!(gathered(&left.into_io_buf()), b"left");
        assert_eq!(gathered(&right.into_io_buf()), b"right");
    }
}
