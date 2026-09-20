use super::{ALIGN, BufPool};

use std::mem::{ManuallyDrop, MaybeUninit};

use aligned_vec::{AVec, ConstAlign};

pub struct Buf {
    pub(super) buf: AVec<u8, ConstAlign<ALIGN>>,
    pub(super) pool: BufPool,
}

pub struct BufIntoIter {
    buf: AVec<u8, ConstAlign<ALIGN>>,
    pool: BufPool,
    start: usize,
    end: usize,
}

impl Buf {
    pub fn new(pool: BufPool) -> Self {
        pool.checkout_owned()
    }

    pub(crate) fn from_avec(avec: AVec<u8, ConstAlign<ALIGN>>, pool: BufPool) -> Self {
        let mut buf = std::mem::ManuallyDrop::new(avec);
        let ptr = buf.as_mut_ptr();
        let len = buf.len();
        let cap = buf.capacity();
        let buf = unsafe { AVec::from_raw_parts(ptr.cast(), ALIGN, len, cap) };
        Buf {
            buf,
            pool,
        }
    }

    pub fn with_capacity(capacity: usize, pool: BufPool) -> Self {
        pool.checkout_with_capacity_owned(capacity)
    }

    pub fn pool(&self) -> &BufPool {
        &self.pool
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.buf.as_ptr().cast()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr().cast()
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.as_mut_ptr().cast(),
                self.capacity().unchecked_sub(self.len()),
            )
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf
            .reserve_exact(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.buf.shrink_to(min_capacity);
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.buf.set_len(new_len);
        }
    }

    pub fn push(&mut self, byte: u8) {
        let len = self.len();
        if len == self.capacity() {
            self.reserve((len * 2).max(1));
        }

        unsafe {
            *self.as_mut_ptr().add(len) = byte;
        }
        unsafe {
            self.set_len(len.unchecked_add(1));
        }
    }

    pub fn pop(&mut self) -> Option<u8> {
        let len = self.len();
        if len == 0 {
            return None;
        }

        let new_len = unsafe { len.unchecked_sub(1) };

        let ptr = unsafe { self.as_mut_ptr().add(new_len) };

        unsafe {
            self.set_len(new_len);
        }

        Some(unsafe { *ptr })
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let slice_len = slice.len();
        let len = self.len();
        let new_len = slice_len + len;
        self.reserve(slice_len.max(len * 2));

        unsafe {
            std::ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.as_mut_ptr().add(len),
                slice.len(),
            );
            self.set_len(new_len);
        }
    }

    pub fn resize(&mut self, new_len: usize, default: u8) {
        let len = self.len();
        if new_len < len {
            unsafe {
                self.set_len(new_len);
            }
        } else if new_len > len {
            let reserve = new_len.max(len * 2);
            self.reserve(unsafe { reserve.unchecked_sub(len) });

            unsafe {
                std::ptr::write_bytes(self.as_mut_ptr().add(len), default, new_len - len);
                self.set_len(new_len);
            }
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        let len = self.len();
        if new_len > len {
            panic!("truncated length is greater than current length");
        }

        unsafe {
            self.set_len(new_len);
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            self.buf.set_len(0);
        }
    }

    pub(crate) fn steal(self) -> AVec<u8, ConstAlign<ALIGN>> {
        let mut this = std::mem::ManuallyDrop::new(self);
        let mut buf =
            std::mem::ManuallyDrop::new(std::mem::replace(&mut this.buf, AVec::new(ALIGN)));
        unsafe { std::ptr::drop_in_place(&mut this.pool) };

        let ptr = buf.as_mut_ptr();
        let len = buf.len();
        let cap = buf.capacity();
        unsafe { AVec::from_raw_parts(ptr.cast(), ALIGN, len, cap) }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        let buf = std::mem::replace(&mut self.buf, AVec::new(ALIGN));
        self.pool.checkin(buf);
    }
}

impl Clone for Buf {
    fn clone(&self) -> Self {
        let mut buf = self.pool.checkout_with_capacity(self.len());
        buf.extend_from_slice(self.as_slice());
        buf
    }
}

impl std::fmt::Debug for Buf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl std::borrow::Borrow<[u8]> for Buf {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::borrow::BorrowMut<[u8]> for Buf {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl AsRef<[u8]> for Buf {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for Buf {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl std::ops::Deref for Buf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::DerefMut for Buf {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl<I> std::ops::Index<I> for Buf
where
    I: std::slice::SliceIndex<[u8]>,
{
    type Output = <I as std::slice::SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<I> std::ops::IndexMut<I> for Buf
where
    I: std::slice::SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

impl<'a> IntoIterator for &'a Buf {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a> IntoIterator for &'a mut Buf {
    type Item = &'a mut u8;
    type IntoIter = std::slice::IterMut<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl IntoIterator for Buf {
    type Item = u8;
    type IntoIter = BufIntoIter;

    fn into_iter(mut self) -> Self::IntoIter {
        let len = self.len();
        unsafe {
            self.set_len(0);
        }

        let this = ManuallyDrop::new(self);
        let pool = unsafe { std::ptr::read(&this.pool) };
        let buf = unsafe { std::ptr::read(&this.buf) };

        BufIntoIter {
            buf,
            pool,
            start: 0,
            end: len,
        }
    }
}

impl BufIntoIter {
    fn as_ptr(&self) -> *const u8 {
        unsafe { self.buf.as_ptr().add(self.start) }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.buf.as_mut_ptr().add(self.start) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }
}

impl Drop for BufIntoIter {
    fn drop(&mut self) {
        let buf = std::mem::replace(&mut self.buf, AVec::new(ALIGN));
        self.pool.checkin(buf);
    }
}

impl Iterator for BufIntoIter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let start = self.start;

        if start == self.end {
            return None;
        }

        self.start += 1;

        Some(unsafe { *self.as_ptr().add(start) })
    }
}

impl DoubleEndedIterator for BufIntoIter {
    fn next_back(&mut self) -> Option<u8> {
        let end = self.end;

        if self.start == end {
            return None;
        }

        let end = unsafe { end.unchecked_sub(1) };
        self.end = end;

        Some(unsafe { *self.as_ptr().add(end) })
    }
}

impl ExactSizeIterator for BufIntoIter {
    fn len(&self) -> usize {
        unsafe { self.end.unchecked_sub(self.start) }
    }
}

impl std::io::Write for Buf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl futures_util::io::AsyncWrite for Buf {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl compio::io::AsyncWrite for Buf {
    async fn write<T>(&mut self, buf: T) -> compio::BufResult<usize, T>
    where
        T: compio::buf::IoBuf,
    {
        let slice = buf.as_init();
        let cnt = slice.len();
        self.extend_from_slice(slice);
        compio::BufResult(Ok(cnt), buf)
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl compio::buf::IoBuf for Buf {
    fn as_init(&self) -> &[u8] {
        self.as_slice()
    }
}

unsafe fn set_len(buf: &mut Buf, new_len: usize) {
    unsafe {
        buf.set_len(new_len);
    }
}

impl compio::buf::SetLen for Buf {
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe { set_len(self, new_len) }
    }
}

impl compio::buf::IoBufMut for Buf {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { std::slice::from_raw_parts_mut(self.buf.as_mut_ptr().cast(), self.buf.capacity()) }
    }

    fn buf_capacity(&mut self) -> usize {
        self.buf.capacity()
    }

    fn buf_mut_ptr(&mut self) -> *mut MaybeUninit<u8> {
        self.buf.as_mut_ptr().cast()
    }

    fn reserve(&mut self, len: usize) -> Result<(), compio::buf::ReserveError> {
        if self.buf.try_reserve(len).is_err() {
            return Err(compio::buf::ReserveError::ReserveFailed(
                "allocation failure".into(),
            ));
        }

        Ok(())
    }

    fn reserve_exact(&mut self, len: usize) -> Result<(), compio::buf::ReserveExactError> {
        if self.buf.capacity() - self.buf.len() >= len {
            return Ok(());
        }

        if self.buf.try_reserve_exact(len).is_err() {
            return Err(compio::buf::ReserveExactError::ReserveFailed(
                "allocation failure".into(),
            ));
        }

        if self.buf.capacity() - self.buf.len() != len {
            return Err(compio::buf::ReserveExactError::ExactSizeMismatch {
                reserved: self.buf.capacity() - self.buf.len(),
                expected: len,
            });
        }
        Ok(())
    }
}
