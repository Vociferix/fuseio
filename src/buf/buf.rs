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

/// Bytes whose first one is at an address that is a multiple of [`ALIGN`].
///
/// That is what makes it sound to reinterpret a prefix as some `T`. [`Buf`]
/// only ever builds one from the base of its [`AVec`], which is aligned by
/// construction, and the `split_*` methods preserve the invariant by refusing
/// any `T` whose size is not itself a multiple of [`ALIGN`].
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
        // SAFETY: `AVec::set_len` asks for exactly what this function asks of
        //         its own caller, so the obligation is passed straight along.
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
        // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`, so the two have
        //         the same layout. The bytes start at the base of an
        //         `AVec<u8, ConstAlign<ALIGN>>`, which satisfies the alignment
        //         `BufSlice` promises.
        unsafe { &*(self.as_bytes() as *const [u8] as *const BufSlice) }
    }

    pub(crate) fn as_slice_mut(&mut self) -> &mut BufSlice {
        // SAFETY: As in `as_slice`, plus the `&mut self` borrow making this the
        //         only reference to the bytes.
        unsafe { &mut *(self.as_mut_bytes() as *mut [u8] as *mut BufSlice) }
    }

    pub(crate) fn try_into_view<R>(self, range: R) -> Option<BufView>
    where
        R: std::ops::RangeBounds<usize>,
    {
        let this = ManuallyDrop::new(self);

        // SAFETY: `this` is wrapped in `ManuallyDrop`, so `Buf::drop` never runs
        //         and neither field is dropped through it. Each is read exactly
        //         once and owned by a local from here on, so nothing is
        //         duplicated. Moving them out by hand is what lets this bypass
        //         `Buf::drop`, which would return the buffer to the pool.
        let buf = unsafe { std::ptr::read(&this.buf) };
        // SAFETY: As above, for the other field.
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
        // SAFETY: `this` is wrapped in `ManuallyDrop`, so neither field is
        //         dropped through it. Each is read once and moves into the
        //         iterator, which checks the buffer back in when it drops.
        let pool = unsafe { std::ptr::read(&this.pool) };
        // SAFETY: As above, for the other field.
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

/// # Safety
///
/// `new_len` must not exceed `buf`'s capacity, and every byte below it must be
/// initialized.
unsafe fn set_len(buf: &mut Buf, new_len: usize) {
    // SAFETY: `Buf::set_len` asks for exactly what this function asks of its
    //         own caller.
    unsafe {
        buf.set_len(new_len);
    }
}

impl compio::buf::SetLen for Buf {
    unsafe fn set_len(&mut self, new_len: usize) {
        // SAFETY: `SetLen::set_len` places the same requirement on its caller
        //         that the free function below does, so it carries over.
        unsafe { set_len(self, new_len) }
    }
}

impl compio::buf::IoBufMut for Buf {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: The allocation really is `capacity()` bytes long, and `&mut
        //         self` makes this the only reference to it. The bytes past
        //         `len()` are uninitialized, which is precisely what
        //         `MaybeUninit<u8>` allows the caller to see.
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
                // SAFETY: The guard above rejected any `T` needing more alignment than
                //         `ALIGN`, and `self` starts `ALIGN`-aligned, so `first`
                //         is aligned for `T`. `split_at_checked` guarantees it is
                //         `size_of::<T>()` bytes long, and `T: AnyBitPattern` means
                //         any contents of those bytes are a valid `T`.
                let first = unsafe { &*(first.as_ptr().cast()) };
                // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`. The remainder
                //         keeps the alignment invariant because the guard above
                //         admits only a `T` whose size is a multiple of `ALIGN`.
                let last = unsafe { &*(last as *const [u8] as *const BufSlice) };

                (first, last)
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
                // SAFETY: The guard above rejected any `T` needing more alignment than
                //         `ALIGN`, and `self` starts `ALIGN`-aligned, so `first`
                //         is aligned for `T`. `split_at_checked` guarantees it is
                //         `size_of::<T>()` bytes long, and `T: AnyBitPattern` means
                //         any contents of those bytes are a valid `T`. The `&mut self`
                //         borrow makes these the only references to the bytes,
                //         and `T: NoUninit` keeps the caller from writing
                //         padding the buffer would then expose.
                let first = unsafe { &mut *(first.as_mut_ptr().cast()) };
                // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`. The remainder
                //         keeps the alignment invariant because the guard above
                //         admits only a `T` whose size is a multiple of `ALIGN`.
                let last = unsafe { &mut *(last as *mut [u8] as *mut BufSlice) };

                (first, last)
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
                // SAFETY: The guard above rejected any `T` needing more alignment than
                //         `ALIGN`, and `self` starts `ALIGN`-aligned, so `first`
                //         is aligned for `T`. `split_at_checked` guarantees it is
                //         `size_of::<T>()` bytes long, and `T: AnyBitPattern` means
                //         any contents of those bytes are a valid `T`. The checked
                //         multiplication above means `first` is exactly `len`
                //         values of `T` long.
                let first = unsafe { std::slice::from_raw_parts(first.as_ptr().cast(), len) };
                // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`. The remainder
                //         keeps the alignment invariant because the guard above
                //         admits only a `T` whose size is a multiple of `ALIGN`.
                let last = unsafe { &*(last as *const [u8] as *const BufSlice) };

                (first, last)
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
                // SAFETY: The guard above rejected any `T` needing more alignment than
                //         `ALIGN`, and `self` starts `ALIGN`-aligned, so `first`
                //         is aligned for `T`. `split_at_checked` guarantees it is
                //         `size_of::<T>()` bytes long, and `T: AnyBitPattern` means
                //         any contents of those bytes are a valid `T`. The checked
                //         multiplication above means `first` is exactly `len`
                //         values of `T` long, the `&mut self` borrow makes
                //         these the only references to the bytes, and
                //         `T: NoUninit` keeps the caller from writing padding
                //         the buffer would then expose.
                let first =
                    unsafe { std::slice::from_raw_parts_mut(first.as_mut_ptr().cast(), len) };
                // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`. The remainder
                //         keeps the alignment invariant because the guard above
                //         admits only a `T` whose size is a multiple of `ALIGN`.
                let last = unsafe { &mut *(last as *mut [u8] as *mut BufSlice) };

                (first, last)
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
        // SAFETY: `BufSlice` is `repr(transparent)` over `[u8]`, so this
        //         rewrites a slice fat pointer into another of the same shape.
        //         An empty `&[u64]` has a length of zero, which is also zero
        //         bytes, and its dangling pointer is `u64`-aligned, so the
        //         result upholds the alignment invariant without owning
        //         anything.
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
        // SAFETY: `view` points into `buf`, which this same struct owns, so the
        //         bytes outlive the borrow. Moving a `BufView` moves only the
        //         `AVec` header; its heap allocation stays put, so the pointer
        //         is not invalidated. Nothing hands out a `&mut` to those bytes.
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
        // SAFETY: `buf` is wrapped in `ManuallyDrop`, so neither field is
        //         dropped through it. Each is read once and moves into the
        //         view, which checks the buffer back in when it drops.
        let pool = unsafe { std::ptr::read(&buf.pool) };
        // SAFETY: As above, for the other field.
        let buf = unsafe { std::ptr::read(&buf.buf) };

        let view = NonNull::from_ref(buf.as_slice());
        BufView { buf, pool, view }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eight bytes, so a prefix of it leaves the remainder aligned.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, bytemuck::AnyBitPattern, bytemuck::NoUninit)]
    struct Eight {
        a: u32,
        b: u32,
    }

    /// Four bytes, so consuming one would leave the remainder unaligned.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, bytemuck::AnyBitPattern, bytemuck::NoUninit)]
    struct Four(u32);

    /// Wants more alignment than the buffer offers.
    #[repr(C, align(16))]
    #[derive(Clone, Copy, Debug, bytemuck::AnyBitPattern, bytemuck::NoUninit)]
    struct OverAligned([u8; 16]);

    fn buf_of(bytes: &[u8]) -> Buf {
        let mut buf = Buf::with_capacity(bytes.len().max(1), BufPool::new());
        buf.extend_from_slice(bytes);
        buf
    }

    fn counted(n: usize) -> Buf {
        buf_of(&(0..n).map(|i| i as u8).collect::<Vec<_>>())
    }

    // The reason `Buf` exists rather than `Vec<u8>`: everything downstream
    // reinterprets prefixes of it as protocol structs.
    #[test]
    fn a_buffer_always_starts_aligned() {
        for capacity in [1, 7, 8, 9, 64, 4096, 128 * 1024] {
            let buf = Buf::with_capacity(capacity, BufPool::new());

            assert_eq!(
                buf.as_ptr() as usize % ALIGN,
                0,
                "capacity {capacity} started unaligned"
            );
        }
    }

    // A buffer that came back from the pool is as aligned as a fresh one.
    #[test]
    fn a_recycled_buffer_is_still_aligned() {
        let pool = BufPool::new();
        drop(pool.checkout_with_capacity(4096));

        let buf = pool.checkout_with_capacity(64);

        assert_eq!(buf.as_ptr() as usize % ALIGN, 0);
    }

    #[test]
    fn the_pool_hands_back_the_allocation_it_was_given() {
        let pool = BufPool::new();

        let first = pool.checkout_with_capacity(4096);
        let capacity = first.capacity();
        let addr = first.as_ptr() as usize;
        drop(first);

        let second = pool.checkout_with_capacity(8);

        assert_eq!(second.capacity(), capacity, "capacity was not reused");
        assert_eq!(second.as_ptr() as usize, addr, "a new allocation was made");
    }

    // An empty buffer has nothing worth keeping, and pooling it would grow the
    // pool without bound for callers that never write anything.
    #[test]
    fn an_empty_buffer_is_not_pooled() {
        let pool = BufPool::new();
        drop(pool.checkout_with_capacity(0));

        let buf = pool.checkout_with_capacity(4096);

        assert!(buf.capacity() >= 4096);
    }

    #[test]
    fn a_recycled_buffer_has_no_leftover_contents() {
        let pool = BufPool::new();
        let mut first = pool.checkout_with_capacity(64);
        first.extend_from_slice(b"stale");
        drop(first);

        let second = pool.checkout_with_capacity(64);

        assert!(second.is_empty(), "{:?}", second.as_bytes());
    }

    // `steal` takes the allocation out of the pool's cycle deliberately, for
    // callers that keep the bytes past the buffer's lifetime.
    #[test]
    fn a_stolen_buffer_does_not_return_to_the_pool() {
        let pool = BufPool::new();
        let mut buf = pool.checkout_with_capacity(64);
        buf.extend_from_slice(b"kept");

        let stolen = buf.steal();

        assert_eq!(stolen.as_slice(), b"kept");
        assert_eq!(
            pool.checkout_with_capacity(0).capacity(),
            0,
            "the stolen allocation came back"
        );
    }

    #[test]
    fn taking_moves_the_bytes_and_leaves_the_buffer_empty() {
        let mut buf = buf_of(b"moved");

        let taken = buf.take();

        assert_eq!(taken.as_bytes(), b"moved");
        assert!(buf.is_empty(), "the original kept its bytes");
    }

    #[test]
    fn a_clone_copies_the_bytes_without_sharing_them() {
        let original = buf_of(b"shared?");
        let mut clone = original.clone();
        clone.push(b'!');

        assert_eq!(original.as_bytes(), b"shared?");
        assert_eq!(clone.as_bytes(), b"shared?!");
    }

    #[test]
    fn splitting_off_a_prefix_reads_it_as_the_type() {
        let buf = counted(16);

        let (eight, rest) = buf.as_slice().split_first::<Eight>().expect("a prefix");

        assert_eq!(
            *eight,
            Eight {
                a: u32::from_ne_bytes([0, 1, 2, 3]),
                b: u32::from_ne_bytes([4, 5, 6, 7]),
            }
        );
        assert_eq!(rest.as_bytes(), &[8, 9, 10, 11, 12, 13, 14, 15]);
    }

    // The invariant the type exists to carry: whatever is handed back is still
    // a legal place to read another struct from.
    #[test]
    fn the_remainder_of_a_split_is_still_aligned() {
        let buf = counted(64);
        let mut rest = buf.as_slice();

        for _ in 0..4 {
            let (_, next) = rest.split_first::<Eight>().expect("a prefix");

            assert_eq!(
                next.as_bytes().as_ptr() as usize % ALIGN,
                0,
                "a split left the remainder unaligned"
            );

            rest = next;
        }
    }

    // Consuming four bytes would leave the remainder four-aligned, so the whole
    // split is refused rather than handing back a `BufSlice` that lies.
    #[test]
    fn a_type_that_would_unalign_the_remainder_is_refused() {
        let buf = counted(64);

        assert!(buf.as_slice().split_first::<Four>().is_none());
        assert!(buf.as_slice().split_slice::<Four>(1).is_none());
    }

    // The buffer is only `ALIGN`-aligned, so it cannot promise more.
    #[test]
    fn a_type_needing_more_alignment_than_the_buffer_is_refused() {
        let buf = counted(64);

        assert!(std::mem::align_of::<OverAligned>() > ALIGN, "test premise");
        assert!(buf.as_slice().split_first::<OverAligned>().is_none());
        assert!(buf.as_slice().split_slice::<OverAligned>(1).is_none());
    }

    #[test]
    fn splitting_more_than_there_is_refuses() {
        let buf = counted(8);

        assert!(buf.as_slice().split_first::<Eight>().is_some());
        assert!(buf.as_slice().split_slice::<Eight>(1).is_some());
        assert!(buf.as_slice().split_slice::<Eight>(2).is_none());

        let short = counted(7);

        assert!(short.as_slice().split_first::<Eight>().is_none());
    }

    #[test]
    fn splitting_a_slice_reads_every_element() {
        let buf = counted(24);

        let (items, rest) = buf.as_slice().split_slice::<Eight>(2).expect("two");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].a, u32::from_ne_bytes([0, 1, 2, 3]));
        assert_eq!(items[1].a, u32::from_ne_bytes([8, 9, 10, 11]));
        assert_eq!(rest.len(), 8);
    }

    #[test]
    fn splitting_zero_elements_consumes_nothing() {
        let buf = counted(16);

        let (items, rest) = buf.as_slice().split_slice::<Eight>(0).expect("none");

        assert!(items.is_empty());
        assert_eq!(rest.len(), 16);
    }

    // A count from the wire times a struct size can overflow `usize`. Wrapping
    // would make a short buffer look long enough for an enormous slice.
    #[test]
    fn a_length_that_overflows_refuses_rather_than_wrapping() {
        let buf = counted(64);
        let absurd = usize::MAX / std::mem::size_of::<Eight>() + 1;

        assert!(buf.as_slice().split_slice::<Eight>(absurd).is_none());
        assert!(buf.as_slice().split_slice::<Eight>(usize::MAX).is_none());
    }

    #[test]
    fn writing_through_a_split_reaches_the_buffer() {
        let mut buf = counted(16);

        {
            let (first, rest) = buf
                .as_slice_mut()
                .split_first_mut::<Eight>()
                .expect("a prefix");

            first.a = u32::from_ne_bytes([0xaa; 4]);
            rest.as_bytes_mut()[0] = 0xbb;
        }

        assert_eq!(&buf.as_bytes()[..4], &[0xaa; 4]);
        assert_eq!(buf.as_bytes()[8], 0xbb);
    }

    #[test]
    fn the_empty_default_slice_is_aligned_and_empty() {
        let empty = <&BufSlice>::default();

        assert!(empty.is_empty());
        assert_eq!(empty.as_bytes().as_ptr() as usize % ALIGN, 0);
    }

    /// The same bytes as [`counted`], as something whose iterator is already
    /// known to honour the `Iterator` contract. These tests are about whether
    /// `BufIntoIter` keeps the same promises, not about how arrays behave.
    fn reference(n: usize) -> std::vec::IntoIter<u8> {
        (0..n).map(|i| i as u8).collect::<Vec<_>>().into_iter()
    }

    #[test]
    fn iterating_yields_every_byte_in_order() {
        assert_eq!(
            counted(8).into_iter().collect::<Vec<_>>(),
            reference(8).collect::<Vec<_>>()
        );
    }

    #[test]
    fn iterating_backwards_yields_every_byte_in_reverse() {
        assert_eq!(
            counted(8).into_iter().rev().collect::<Vec<_>>(),
            reference(8).rev().collect::<Vec<_>>()
        );
    }

    // Both ends walk towards each other and stop when they meet, rather than
    // crossing over and yielding a byte twice.
    #[test]
    fn the_two_ends_meet_without_overlapping() {
        let mut ours = counted(8).into_iter();
        let mut theirs = reference(8);

        for _ in 0..4 {
            assert_eq!(ours.next(), theirs.next());
            assert_eq!(ours.next_back(), theirs.next_back());
        }

        assert_eq!(ours.next(), None);
        assert_eq!(ours.next_back(), None);
    }

    // `nth` skips and consumes; past the end it consumes everything, so the
    // iterator is finished afterwards.
    #[test]
    fn nth_skips_consumes_and_exhausts() {
        for n in 0..12 {
            let mut ours = counted(8).into_iter();
            let mut theirs = reference(8);

            assert_eq!(ours.nth(n), theirs.nth(n), "nth({n})");
            assert_eq!(ours.next(), theirs.next(), "after nth({n})");
            assert_eq!(ours.len(), theirs.len(), "remaining after nth({n})");
        }
    }

    #[test]
    fn nth_back_skips_consumes_and_exhausts() {
        for n in 0..12 {
            let mut ours = counted(8).into_iter();
            let mut theirs = reference(8);

            assert_eq!(ours.nth_back(n), theirs.nth_back(n), "nth_back({n})");
            assert_eq!(ours.next_back(), theirs.next_back(), "after nth_back({n})");
            assert_eq!(ours.len(), theirs.len(), "remaining after nth_back({n})");
        }
    }

    // `size_hint` and `len` are promises about how many bytes are left, which
    // callers may rely on to allocate.
    #[test]
    fn the_reported_length_tracks_what_is_left() {
        let mut it = counted(8).into_iter();

        for expected in (0..=8).rev() {
            assert_eq!(it.len(), expected);
            assert_eq!(it.size_hint(), (expected, Some(expected)));

            if it.next().is_none() {
                assert_eq!(expected, 0);
            }
        }
    }

    // Declared `FusedIterator`, which promises `None` forever once exhausted.
    #[test]
    fn an_exhausted_iterator_stays_exhausted() {
        let mut it = counted(2).into_iter();

        assert!(it.by_ref().take(2).count() == 2);

        for _ in 0..3 {
            assert_eq!(it.next(), None);
            assert_eq!(it.next_back(), None);
        }
    }

    #[test]
    fn the_overridden_adaptors_agree_with_plain_iteration() {
        assert_eq!(counted(8).into_iter().count(), 8);
        assert_eq!(counted(8).into_iter().last(), Some(7));
        assert_eq!(counted(0).into_iter().last(), None);
        assert_eq!(counted(8).into_iter().fold(0u32, |a, b| a + b as u32), 28);
        assert_eq!(counted(8).into_iter().rfold(0u32, |a, b| a + b as u32), 28);
    }

    // Partly consumed from both ends, the remaining slice is what is left and
    // nothing more.
    #[test]
    fn the_slice_view_shows_only_what_is_unconsumed() {
        let mut it = counted(8).into_iter();
        it.next();
        it.next_back();

        assert_eq!(it.as_slice(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(it.as_mut_slice(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_view_shows_the_range_it_was_made_from() {
        let view = counted(16).try_into_view(4..12).expect("in range");

        assert_eq!(view.len(), 8);
        assert_eq!(view.as_bytes(), &[4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn a_view_of_everything_is_the_whole_buffer() {
        let view = counted(16).try_into_view(..).expect("in range");

        assert_eq!(view.as_bytes(), counted(16).as_bytes());
    }

    #[test]
    fn a_view_outside_the_buffer_is_refused() {
        assert!(counted(16).try_into_view(4..17).is_none());
        assert!(counted(16).try_into_view(17..).is_none());
    }

    // The buffer belongs to the pool whether or not the range was usable, so a
    // refused view must not cost the pool its allocation.
    #[test]
    fn a_refused_view_returns_the_buffer_to_the_pool() {
        let pool = BufPool::new();
        let mut buf = pool.checkout_with_capacity(4096);
        buf.resize(16, 0);
        let capacity = buf.capacity();

        assert!(buf.try_into_view(0..999).is_none());

        assert_eq!(
            pool.checkout_with_capacity(0).capacity(),
            capacity,
            "the pool lost the allocation"
        );
    }

    #[test]
    fn a_dropped_view_returns_the_buffer_to_the_pool() {
        let pool = BufPool::new();
        let mut buf = pool.checkout_with_capacity(4096);
        buf.resize(16, 0);
        let capacity = buf.capacity();

        drop(buf.try_into_view(0..8).expect("in range"));

        assert_eq!(pool.checkout_with_capacity(0).capacity(), capacity);
    }

    #[test]
    fn a_whole_buffer_view_keeps_the_bytes() {
        let view = BufView::from(buf_of(b"all of it"));

        assert_eq!(view.as_bytes(), b"all of it");
        assert_eq!(view.as_os_str(), std::ffi::OsStr::new("all of it"));
    }
}
