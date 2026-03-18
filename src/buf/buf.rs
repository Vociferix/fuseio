use super::{ALIGN, BufPool};

use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};

use aligned_vec::{AVec, ConstAlign};

pub struct Buf<T = u8> {
    pub(super) buf: AVec<u8, ConstAlign<ALIGN>>,
    pub(super) pool: BufPool,
    pub(super) _phantom: PhantomData<AVec<T, ConstAlign<ALIGN>>>,
}

pub struct BufIntoIter<T = u8> {
    buf: AVec<u8, ConstAlign<ALIGN>>,
    pool: BufPool,
    start: usize,
    end: usize,
    _phantom: PhantomData<AVec<T, ConstAlign<ALIGN>>>,
}

impl<T> Buf<T> {
    pub fn new(pool: BufPool) -> Self {
        pool.checkout_owned()
    }

    pub fn with_capacity(capacity: usize, pool: BufPool) -> Self {
        pool.checkout_with_capacity_owned(capacity)
    }

    pub fn pool(&self) -> &BufPool {
        &self.pool
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity() / std::mem::size_of::<T>()
    }

    pub fn len(&self) -> usize {
        self.buf.len() / std::mem::size_of::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_ptr(&self) -> *const T {
        self.buf.as_ptr().cast()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr().cast()
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.as_mut_ptr().cast(),
                self.capacity().unchecked_sub(self.len()),
            )
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional * std::mem::size_of::<T>());
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf
            .reserve_exact(additional * std::mem::size_of::<T>());
    }

    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.buf.shrink_to(min_capacity * std::mem::size_of::<T>());
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.buf.set_len(new_len * std::mem::size_of::<T>());
        }
    }

    pub fn push(&mut self, item: T) -> &mut T {
        let len = self.len();
        if len == self.capacity() {
            self.reserve((len * 2).max(1));
        }

        let ptr = unsafe { self.as_mut_ptr().add(len) };
        unsafe {
            std::ptr::write(ptr, item);
        }
        unsafe {
            self.set_len(len.unchecked_add(1));
        }

        unsafe { &mut *ptr }
    }

    pub fn pop(&mut self) -> Option<T> {
        let len = self.len();
        if len == 0 {
            return None;
        }

        let new_len = unsafe { len.unchecked_sub(1) };

        let ptr = unsafe { self.as_mut_ptr().add(new_len) };

        unsafe {
            self.set_len(new_len);
        }

        Some(unsafe { std::ptr::read(ptr) })
    }

    pub fn extend_from_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        let slice_len = slice.len();
        let len = self.len();
        let new_len = slice_len + len;
        self.reserve(slice_len.max(len * 2));

        if is_copy::<T>() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    slice.as_ptr(),
                    self.as_mut_ptr().add(len),
                    slice.len(),
                );
                self.set_len(new_len);
            }
        } else {
            let dst_ptr = unsafe { self.as_mut_ptr().add(len) };
            let src_ptr = slice.as_ptr();
            for idx in 0..slice_len {
                unsafe {
                    std::ptr::write(dst_ptr.add(idx), (*src_ptr.add(idx)).clone());
                    self.set_len(len.unchecked_add(idx));
                }
            }
        }
    }

    pub fn resize(&mut self, new_len: usize, default: T)
    where
        T: Clone,
    {
        let len = self.len();
        if new_len < len {
            if std::mem::needs_drop::<T>() {
                unsafe {
                    std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                        self.as_mut_ptr().add(new_len),
                        len.unchecked_sub(new_len),
                    ));
                }
            }
            unsafe {
                self.set_len(new_len);
            }
        } else if new_len > len {
            let reserve = new_len.max(len * 2);
            self.reserve(unsafe { reserve.unchecked_sub(len) });

            let ptr = self.as_mut_ptr();
            for idx in len..new_len {
                unsafe {
                    std::ptr::write(ptr.add(idx), default.clone());
                    self.set_len(idx.unchecked_add(1));
                }
            }
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        let len = self.len();
        if new_len > len {
            panic!("truncated length is greater than current length");
        }

        if std::mem::needs_drop::<T>() {
            unsafe {
                std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                    self.as_mut_ptr().add(new_len),
                    len.unchecked_sub(new_len),
                ));
            }
        }
        unsafe {
            self.set_len(new_len);
        }
    }

    pub fn clear(&mut self) {
        if std::mem::needs_drop::<T>() {
            unsafe {
                std::ptr::drop_in_place(self.as_mut_slice());
            }
        }
        unsafe {
            self.buf.set_len(0);
        }
    }
}

impl<T: bytemuck::Pod> Buf<T> {
    pub fn try_cast<U: bytemuck::Pod>(self) -> Result<Buf<U>, Self> {
        if std::mem::align_of::<U>() > ALIGN || self.buf.len() % std::mem::size_of::<U>() != 0 {
            Err(self)
        } else {
            unsafe { Ok(self.cast_unchecked()) }
        }
    }

    pub unsafe fn cast_unchecked<U: bytemuck::Pod>(self) -> Buf<U> {
        let this = ManuallyDrop::new(self);
        let buf = unsafe { std::ptr::read(&this.buf) };
        let pool = unsafe { std::ptr::read(&this.pool) };
        Buf {
            buf,
            pool,
            _phantom: PhantomData,
        }
    }

    pub fn cast<U: bytemuck::Pod>(self) -> Buf<U> {
        if std::mem::align_of::<U>() > ALIGN {
            panic!(
                "type {} does not satisfy alignment requirements",
                std::any::type_name::<U>()
            );
        }
        if self.buf.len() % std::mem::size_of::<U>() != 0 {
            panic!(
                "type {} does not fit evenly into buffer",
                std::any::type_name::<U>()
            );
        }
        unsafe { self.cast_unchecked() }
    }
}

impl<T> Drop for Buf<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            let ptr = self.as_mut_ptr();
            let len = self.len();
            unsafe {
                self.set_len(0);
                std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(ptr, len));
            }
        }

        let buf = std::mem::replace(&mut self.buf, AVec::new(ALIGN));
        self.pool.checkin(buf);
    }
}

impl<T: Clone> Clone for Buf<T> {
    fn clone(&self) -> Self {
        let mut buf = self.pool.checkout_with_capacity::<T>(self.len());
        buf.extend_from_slice(self.as_slice());
        buf
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Buf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T> std::borrow::Borrow<[T]> for Buf<T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> std::borrow::BorrowMut<[T]> for Buf<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> AsRef<[T]> for Buf<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for Buf<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> std::ops::Deref for Buf<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> std::ops::DerefMut for Buf<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, I> std::ops::Index<I> for Buf<T>
where
    I: std::slice::SliceIndex<[T]>,
{
    type Output = <I as std::slice::SliceIndex<[T]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T, I> std::ops::IndexMut<I> for Buf<T>
where
    I: std::slice::SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

impl<'a, T> IntoIterator for &'a Buf<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Buf<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T> IntoIterator for Buf<T> {
    type Item = T;
    type IntoIter = BufIntoIter<T>;

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
            _phantom: PhantomData,
        }
    }
}

impl<T> BufIntoIter<T> {
    fn as_ptr(&self) -> *const T {
        unsafe { self.buf.as_ptr().cast::<T>().add(self.start) }
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { self.buf.as_mut_ptr().cast::<T>().add(self.start) }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }
}

impl<T> Drop for BufIntoIter<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            unsafe {
                std::ptr::drop_in_place(self.as_mut_slice());
            }
        }

        let buf = std::mem::replace(&mut self.buf, AVec::new(ALIGN));
        self.pool.checkin(buf);
    }
}

impl<T> Iterator for BufIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            return None;
        }

        let start = self.start;
        let item = unsafe { std::ptr::read(self.buf.as_mut_ptr().cast::<T>().add(start)) };
        self.start = unsafe { start.unchecked_add(1) };
        Some(item)
    }
}

impl<T> DoubleEndedIterator for BufIntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            return None;
        }

        let end = unsafe { self.end.unchecked_add(1) };
        let item = unsafe { std::ptr::read(self.buf.as_mut_ptr().cast::<T>().add(end)) };
        self.end = end;
        Some(item)
    }
}

impl<T> ExactSizeIterator for BufIntoIter<T> {
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

#[inline(always)]
fn is_copy<T>() -> bool {
    struct IsCopy<'a, T> {
        is_copy: &'a std::cell::Cell<bool>,
        _marker: PhantomData<T>,
    }

    impl<T> Clone for IsCopy<'_, T> {
        fn clone(&self) -> Self {
            self.is_copy.set(false);
            IsCopy {
                is_copy: self.is_copy,
                _marker: PhantomData,
            }
        }
    }

    impl<T: Copy> Copy for IsCopy<'_, T> {}

    let is_copy = std::cell::Cell::new(true);
    let _ = [IsCopy::<T> {
        is_copy: &is_copy,
        _marker: PhantomData,
    }]
    .clone();
    is_copy.get()
}
