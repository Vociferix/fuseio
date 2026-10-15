use super::{ALIGN, BufPool};

use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;

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

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BufSlice([u8]);

pub struct BufView {
    buf: AVec<u8, ConstAlign<ALIGN>>,
    pool: BufPool,
    view: NonNull<[u8]>,
}

impl Buf {
    pub fn new(pool: BufPool) -> Self {
        pool.checkout_owned()
    }

    pub(crate) fn from_avec(avec: AVec<u8, ConstAlign<ALIGN>>, pool: BufPool) -> Self {
        Buf { buf: avec, pool }
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

    pub fn as_bytes(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve_exact(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.buf.shrink_to(min_capacity);
    }

    /// # Safety
    ///
    /// `new_len` must not exceed the buffer's capacity, and every byte below it
    /// must be initialized.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.buf.set_len(new_len);
        }
    }

    pub fn push(&mut self, byte: u8) {
        self.buf.push(byte);
    }

    pub fn pop(&mut self) -> Option<u8> {
        self.buf.pop()
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.buf.extend_from_slice(slice);
    }

    pub fn resize(&mut self, new_len: usize, default: u8) {
        self.buf.resize(new_len, default);
    }

    pub fn truncate(&mut self, new_len: usize) {
        self.buf.truncate(new_len);
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn take(&mut self) -> Self {
        let pool = self.pool.clone();
        Self {
            buf: std::mem::replace(&mut self.buf, AVec::new(ALIGN)),
            pool,
        }
    }

    pub(crate) fn steal(mut self) -> AVec<u8, ConstAlign<ALIGN>> {
        std::mem::replace(&mut self.buf, AVec::new(ALIGN))
    }

    pub(crate) fn as_slice(&self) -> &BufSlice {
        unsafe { &*(self.as_bytes() as *const [u8] as *const BufSlice) }
    }

    pub(crate) fn as_slice_mut(&mut self) -> &mut BufSlice {
        unsafe { &mut *(self.as_mut_bytes() as *mut [u8] as *mut BufSlice) }
    }

    pub(crate) fn try_into_view<R>(self, range: R) -> Option<BufView>
    where
        R: std::ops::RangeBounds<usize>,
    {
        let this = ManuallyDrop::new(self);

        let buf = unsafe { std::ptr::read(&this.buf) };
        let pool = unsafe { std::ptr::read(&this.pool) };

        let Some(view) = buf.get((range.start_bound().cloned(), range.end_bound().cloned())) else {
            pool.checkin(buf);
            return None;
        };

        let view = NonNull::from_ref(view);

        Some(BufView { buf, pool, view })
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
        buf.extend_from_slice(self.as_bytes());
        buf
    }
}

impl std::fmt::Debug for Buf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_bytes(), f)
    }
}

impl std::borrow::Borrow<[u8]> for Buf {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl std::borrow::BorrowMut<[u8]> for Buf {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_mut_bytes()
    }
}

impl AsRef<[u8]> for Buf {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsMut<[u8]> for Buf {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_bytes()
    }
}

impl std::ops::Deref for Buf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl std::ops::DerefMut for Buf {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.as_mut_bytes()
    }
}

impl<I> std::ops::Index<I> for Buf
where
    I: std::slice::SliceIndex<[u8]>,
{
    type Output = <I as std::slice::SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.as_bytes().index(index)
    }
}

impl<I> std::ops::IndexMut<I> for Buf
where
    I: std::slice::SliceIndex<[u8]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_bytes().index_mut(index)
    }
}

impl<'a> IntoIterator for &'a Buf {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_bytes().iter()
    }
}

impl<'a> IntoIterator for &'a mut Buf {
    type Item = &'a mut u8;
    type IntoIter = std::slice::IterMut<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_bytes().iter_mut()
    }
}

impl IntoIterator for Buf {
    type Item = u8;
    type IntoIter = BufIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let len = self.len();
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
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf[self.start..self.end]
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
        let &byte = self.as_slice().first()?;
        self.start += 1;
        Some(byte)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn nth(&mut self, n: usize) -> Option<u8> {
        let Some(&byte) = self.as_slice().get(n) else {
            self.start = self.end;
            return None;
        };
        self.start += n + 1;
        Some(byte)
    }

    fn last(mut self) -> Option<u8> {
        self.next_back()
    }

    fn count(self) -> usize {
        self.len()
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.as_slice()
            .iter()
            .fold(init, move |acc, &byte| f(acc, byte))
    }
}

impl DoubleEndedIterator for BufIntoIter {
    fn next_back(&mut self) -> Option<u8> {
        let &byte = self.as_slice().last()?;
        self.end -= 1;
        Some(byte)
    }

    fn nth_back(&mut self, n: usize) -> Option<u8> {
        let Some(&byte) = self
            .len()
            .checked_sub(n)
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.as_slice().get(i))
        else {
            self.end = self.start;
            return None;
        };
        self.end -= n + 1;
        Some(byte)
    }

    fn rfold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.as_slice()
            .iter()
            .rfold(init, move |acc, &byte| f(acc, byte))
    }
}

impl ExactSizeIterator for BufIntoIter {
    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl std::iter::FusedIterator for BufIntoIter {}

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
        self.as_bytes()
    }

    fn buf_len(&self) -> usize {
        self.len()
    }

    fn buf_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
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

impl BufSlice {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    pub fn split_first<T>(&self) -> Option<(&T, &Self)>
    where
        T: bytemuck::AnyBitPattern,
    {
        if std::mem::align_of::<T>() > ALIGN || !std::mem::size_of::<T>().is_multiple_of(ALIGN) {
            return None;
        }

        self.0
            .split_at_checked(std::mem::size_of::<T>())
            .map(|(first, last)| {
                (unsafe { &*(first.as_ptr().cast()) }, unsafe {
                    &*(last as *const [u8] as *const BufSlice)
                })
            })
    }

    pub fn split_first_mut<T>(&mut self) -> Option<(&mut T, &mut Self)>
    where
        T: bytemuck::AnyBitPattern + bytemuck::NoUninit,
    {
        if std::mem::align_of::<T>() > ALIGN || !std::mem::size_of::<T>().is_multiple_of(ALIGN) {
            return None;
        }

        self.0
            .split_at_mut_checked(std::mem::size_of::<T>())
            .map(|(first, last)| {
                (unsafe { &mut *(first.as_mut_ptr().cast()) }, unsafe {
                    &mut *(last as *mut [u8] as *mut BufSlice)
                })
            })
    }

    pub fn split_slice<T>(&self, len: usize) -> Option<(&[T], &Self)>
    where
        T: bytemuck::AnyBitPattern,
    {
        if std::mem::align_of::<T>() > ALIGN || !std::mem::size_of::<T>().is_multiple_of(ALIGN) {
            return None;
        }

        self.0
            .split_at_checked(len.checked_mul(std::mem::size_of::<T>())?)
            .map(|(first, last)| {
                (
                    unsafe { std::slice::from_raw_parts(first.as_ptr().cast(), len) },
                    unsafe { &*(last as *const [u8] as *const BufSlice) },
                )
            })
    }

    pub fn split_slice_mut<T>(&mut self, len: usize) -> Option<(&mut [T], &mut Self)>
    where
        T: bytemuck::AnyBitPattern + bytemuck::NoUninit,
    {
        if std::mem::align_of::<T>() > ALIGN || !std::mem::size_of::<T>().is_multiple_of(ALIGN) {
            return None;
        }

        self.0
            .split_at_mut_checked(len.checked_mul(std::mem::size_of::<T>())?)
            .map(|(first, last)| {
                (
                    unsafe { std::slice::from_raw_parts_mut(first.as_mut_ptr().cast(), len) },
                    unsafe { &mut *(last as *mut [u8] as *mut BufSlice) },
                )
            })
    }
}

impl std::ops::Deref for BufSlice {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BufSlice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::borrow::Borrow<[u8]> for BufSlice {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl std::borrow::BorrowMut<[u8]> for BufSlice {
    fn borrow_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl AsRef<[u8]> for BufSlice {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for BufSlice {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl std::fmt::Debug for BufSlice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl Default for &BufSlice {
    fn default() -> Self {
        const DEFAULT: &BufSlice = unsafe {
            let aligned_slice: &'static [u64] = &[];
            std::mem::transmute(aligned_slice)
        };
        DEFAULT
    }
}

impl BufView {
    pub fn len(&self) -> usize {
        self.view.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.view.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.view.as_ref() }
    }

    pub fn as_os_str(&self) -> &std::ffi::OsStr {
        std::os::unix::ffi::OsStrExt::from_bytes(self.as_bytes())
    }
}

impl std::ops::Deref for BufView {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::borrow::Borrow<[u8]> for BufView {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<[u8]> for BufView {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Clone for BufView {
    fn clone(&self) -> Self {
        let mut buf = self.pool.checkout_with_capacity(self.len());
        buf.extend_from_slice(self);
        buf.into()
    }

    fn clone_from(&mut self, other: &Self) {
        self.buf.clear();
        self.buf.extend_from_slice(other);
        self.view = NonNull::from_ref(self.buf.as_slice());
    }
}

impl std::fmt::Debug for BufView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_bytes(), f)
    }
}

impl PartialEq for BufView {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for BufView {}

impl PartialOrd for BufView {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for BufView {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(self.as_bytes(), other.as_bytes())
    }
}

impl std::hash::Hash for BufView {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.as_bytes().hash(state);
    }
}

impl Drop for BufView {
    fn drop(&mut self) {
        let buf = std::mem::replace(&mut self.buf, AVec::new(ALIGN));
        self.pool.checkin(buf);
    }
}

impl From<Buf> for BufView {
    fn from(buf: Buf) -> Self {
        let buf = ManuallyDrop::new(buf);
        let pool = unsafe { std::ptr::read(&buf.pool) };
        let buf = unsafe { std::ptr::read(&buf.buf) };

        let view = NonNull::from_ref(buf.as_slice());
        BufView { buf, pool, view }
    }
}
