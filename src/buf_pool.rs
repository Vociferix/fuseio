use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::rc::Rc;

use aligned_vec::{AVec, ConstAlign};
use bytemuck::{Pod, bytes_of, cast_slice};

const ALIGN: usize = std::mem::align_of::<u64>();

#[derive(Debug)]
pub struct BufPool {
    pool: Rc<UnsafeCell<Vec<AVec<u8, ConstAlign<ALIGN>>>>>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Buf<T> {
    buf: AVec<u8, ConstAlign<ALIGN>>,
    _phantom: PhantomData<AVec<T, ConstAlign<ALIGN>>>,
}

#[derive(Debug, Clone)]
pub struct BufGuard<T: Pod> {
    buf: Buf<T>,
    pool: BufPool,
}

#[derive(Debug)]
struct ReserveError;

impl BufPool {
    pub fn new() -> Self {
        Self {
            pool: Rc::new(UnsafeCell::new(Vec::new())),
        }
    }

    pub fn checkout<T: Pod>(&self) -> BufGuard<T> {
        assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());
        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = match buf_opt {
            Some(buf) => Buf {
                buf,
                _phantom: PhantomData,
            },
            None => Buf::new(),
        };
        BufGuard {
            buf,
            pool: self.clone(),
        }
    }

    pub fn checkout_with_capacity<T: Pod>(&self, capacity: usize) -> BufGuard<T> {
        assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());
        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = match buf_opt {
            Some(mut buf) => {
                buf.reserve(capacity * std::mem::size_of::<T>());
                Buf {
                    buf,
                    _phantom: PhantomData,
                }
            }
            None => Buf::with_capacity(capacity),
        };
        BufGuard {
            buf,
            pool: self.clone(),
        }
    }

    fn checkin<T: Copy>(&self, buf: Buf<T>) {
        let mut buf = buf.buf;
        if buf.capacity() > 0 {
            buf.clear();
            unsafe {
                (*self.pool.get()).push(buf);
            }
        }
    }
}

impl Clone for BufPool {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl<T: Pod> BufGuard<T> {
    pub fn cast<U: Pod>(self) -> BufGuard<U> {
        let this = std::mem::ManuallyDrop::new(self);
        let buf = unsafe { std::ptr::read(&raw const this.buf) };
        let pool = unsafe { std::ptr::read(&raw const this.pool) };
        BufGuard {
            buf: buf.cast(),
            pool,
        }
    }

    pub fn try_cast<U: Pod>(self) -> Result<BufGuard<U>, Self> {
        let this = std::mem::ManuallyDrop::new(self);
        let buf = unsafe { std::ptr::read(&raw const this.buf) };
        let pool = unsafe { std::ptr::read(&raw const this.pool) };
        match buf.try_cast() {
            Ok(buf) => Ok(BufGuard { buf, pool }),
            Err(buf) => Err(BufGuard { buf, pool }),
        }
    }

    pub unsafe fn cast_unchecked<U: Pod>(self) -> BufGuard<U> {
        let this = std::mem::ManuallyDrop::new(self);
        let buf = unsafe { std::ptr::read(&raw const this.buf) };
        let pool = unsafe { std::ptr::read(&raw const this.pool) };
        BufGuard {
            buf: unsafe { buf.cast_unchecked() },
            pool,
        }
    }
}

impl<T: Pod> Drop for BufGuard<T> {
    fn drop(&mut self) {
        let buf = std::mem::take(&mut self.buf);
        self.pool.checkin(buf);
    }
}

impl<T: Pod> std::ops::Deref for BufGuard<T> {
    type Target = Buf<T>;

    fn deref(&self) -> &Buf<T> {
        &self.buf
    }
}

impl<T: Pod> std::ops::DerefMut for BufGuard<T> {
    fn deref_mut(&mut self) -> &mut Buf<T> {
        &mut self.buf
    }
}

impl<T: Pod> Buf<T> {
    pub fn new() -> Self {
        assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());
        Self {
            buf: AVec::new(ALIGN),
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());
        Self {
            buf: AVec::with_capacity(ALIGN, capacity * std::mem::size_of::<T>()),
            _phantom: PhantomData,
        }
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
        self.buf.as_ptr() as *const T
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr() as *mut T
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    pub fn reserve(&mut self, additional: usize) {
        let additional = additional * std::mem::size_of::<T>();
        let total = (self.buf.len() + additional).max(self.buf.len() * 2);
        if total <= self.buf.capacity() {
            return;
        }
        self.buf.reserve(total - self.buf.len());
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf
            .reserve_exact(additional * std::mem::size_of::<T>());
    }

    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit();
    }

    pub fn push(&mut self, item: T) {
        self.buf.extend_from_slice(bytes_of(&item));
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let new_len = self.len() - 1;

        let item = unsafe {
            let item = *self.as_ptr().add(new_len);
            self.buf.set_len(new_len * std::mem::size_of::<T>());
            item
        };

        Some(item)
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        self.buf.extend_from_slice(cast_slice(slice));
    }

    pub fn resize(&mut self, new_len: usize, default: T) {
        let len = self.len();

        if new_len > len {
            self.reserve_exact(new_len - len);
            let ptr = self.as_mut_ptr();
            for idx in len..new_len {
                unsafe {
                    std::ptr::write(ptr.add(idx), default);
                }
            }
        }

        unsafe {
            self.buf.set_len(new_len * std::mem::size_of::<T>());
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        if new_len > self.len() {
            panic!("Buf::truncate new length is larger than current length");
        }

        unsafe {
            self.buf.set_len(new_len * std::mem::size_of::<T>());
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.buf.set_len(new_len * std::mem::size_of::<T>());
        }
    }

    pub fn cast<U: Pod>(self) -> Buf<U> {
        if self.buf.len() % std::mem::size_of::<U>() != 0 {
            panic!(
                "Buf::cast cannot cast [{}; {}] to [{}]",
                std::any::type_name::<T>(),
                self.len(),
                std::any::type_name::<U>()
            );
        }

        let Self { buf, .. } = self;
        Buf {
            buf,
            _phantom: PhantomData,
        }
    }

    pub fn try_cast<U: Pod>(self) -> Result<Buf<U>, Self> {
        if self.buf.len() % std::mem::size_of::<U>() == 0 {
            let Self { buf, .. } = self;
            Ok(Buf {
                buf,
                _phantom: PhantomData,
            })
        } else {
            Err(self)
        }
    }

    pub unsafe fn cast_unchecked<U: Pod>(self) -> Buf<U> {
        let Self { buf, .. } = self;
        Buf {
            buf,
            _phantom: PhantomData,
        }
    }
}

impl<T: Pod> Default for Buf<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Pod + std::fmt::Debug> std::fmt::Debug for Buf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T: Pod> std::ops::Deref for Buf<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Pod> std::ops::DerefMut for Buf<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Pod> std::borrow::Borrow<[T]> for Buf<T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Pod> std::borrow::BorrowMut<[T]> for Buf<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Pod> AsRef<[T]> for Buf<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Pod> AsMut<[T]> for Buf<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Pod + Eq> Eq for Buf<T> {}

impl<T: Pod + PartialEq> PartialEq for Buf<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_slice(), other.as_slice())
    }
}

impl<T: Pod + PartialEq> PartialEq<[T]> for Buf<T> {
    fn eq(&self, other: &[T]) -> bool {
        PartialEq::eq(self.as_slice(), other)
    }
}

impl<T: Pod + PartialEq> PartialEq<Buf<T>> for [T] {
    fn eq(&self, other: &Buf<T>) -> bool {
        PartialEq::eq(self, other.as_slice())
    }
}

impl<T: Pod + PartialEq> PartialEq<&[T]> for Buf<T> {
    fn eq(&self, other: &&[T]) -> bool {
        PartialEq::eq(self, *other)
    }
}

impl<T: Pod + PartialEq> PartialEq<Buf<T>> for &[T] {
    fn eq(&self, other: &Buf<T>) -> bool {
        PartialEq::eq(*self, other)
    }
}

impl<T: Pod + PartialEq> PartialEq<[T]> for &Buf<T> {
    fn eq(&self, other: &[T]) -> bool {
        PartialEq::eq(*self, other)
    }
}

impl<T: Pod + PartialEq> PartialEq<&Buf<T>> for [T] {
    fn eq(&self, other: &&Buf<T>) -> bool {
        PartialEq::eq(self, *other)
    }
}

impl<T: Pod + Ord> Ord for Buf<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Pod + PartialOrd> PartialOrd for Buf<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Pod + PartialOrd> PartialOrd<[T]> for Buf<T> {
    fn partial_cmp(&self, other: &[T]) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other)
    }
}

impl<T: Pod + PartialOrd> PartialOrd<Buf<T>> for [T] {
    fn partial_cmp(&self, other: &Buf<T>) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self, other.as_slice())
    }
}

impl<T: Pod + PartialOrd> PartialOrd<&[T]> for Buf<T> {
    fn partial_cmp(&self, other: &&[T]) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self, *other)
    }
}

impl<T: Pod + PartialOrd> PartialOrd<Buf<T>> for &[T] {
    fn partial_cmp(&self, other: &Buf<T>) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(*self, other)
    }
}

impl<T: Pod + PartialOrd> PartialOrd<[T]> for &Buf<T> {
    fn partial_cmp(&self, other: &[T]) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(*self, other)
    }
}

impl<T: Pod + PartialOrd> PartialOrd<&Buf<T>> for [T] {
    fn partial_cmp(&self, other: &&Buf<T>) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self, *other)
    }
}

impl<T: Pod + std::hash::Hash> std::hash::Hash for Buf<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.as_slice().hash(state)
    }
}

impl<'a, T: Pod> IntoIterator for &'a Buf<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T: Pod> IntoIterator for &'a mut Buf<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T: Pod> Extend<T> for Buf<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let iter = iter.into_iter();
        let count = iter.size_hint().0;
        self.reserve(count);
        iter.for_each(|item| self.push(item));
    }
}

impl<'a, T: Pod> Extend<&'a T> for Buf<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a T>,
    {
        self.extend(iter.into_iter().copied());
    }
}

impl<'a, T: Pod> Extend<&'a mut T> for Buf<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a mut T>,
    {
        self.extend(iter.into_iter().map(|item| *item));
    }
}

impl compio::buf::SetLen for Buf<u8> {
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.buf.set_len(new_len);
        }
    }
}

impl compio::buf::SetLen for BufGuard<u8> {
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            compio::buf::SetLen::set_len(&mut **self, new_len);
        }
    }
}

impl<T: Pod> compio::buf::IoBuf for Buf<T> {
    fn as_init(&self) -> &[u8] {
        self.buf.as_slice()
    }
}

impl<T: Pod> compio::buf::IoBuf for BufGuard<T> {
    fn as_init(&self) -> &[u8] {
        compio::buf::IoBuf::as_init(&**self)
    }
}

impl compio::buf::IoBufMut for Buf<u8> {
    fn as_uninit(&mut self) -> &mut [std::mem::MaybeUninit<u8>] {
        unsafe { std::slice::from_raw_parts_mut(self.buf.as_mut_ptr().cast(), self.buf.capacity()) }
    }

    fn buf_capacity(&mut self) -> usize {
        self.buf.capacity()
    }

    fn buf_mut_ptr(&mut self) -> *mut std::mem::MaybeUninit<u8> {
        self.buf.as_mut_ptr().cast()
    }

    fn reserve(&mut self, len: usize) -> Result<(), compio::buf::ReserveError> {
        self.buf
            .try_reserve(len)
            .map_err(|err| compio::buf::ReserveError::ReserveFailed(Box::new(ReserveError)))
    }

    fn reserve_exact(&mut self, len: usize) -> Result<(), compio::buf::ReserveExactError> {
        self.buf
            .try_reserve_exact(len)
            .map_err(|err| compio::buf::ReserveExactError::ReserveFailed(Box::new(ReserveError)))
    }
}

impl compio::buf::IoBufMut for BufGuard<u8> {
    fn as_uninit(&mut self) -> &mut [std::mem::MaybeUninit<u8>] {
        compio::buf::IoBufMut::as_uninit(&mut **self)
    }

    fn buf_capacity(&mut self) -> usize {
        compio::buf::IoBufMut::buf_capacity(&mut **self)
    }

    fn buf_mut_ptr(&mut self) -> *mut std::mem::MaybeUninit<u8> {
        compio::buf::IoBufMut::buf_mut_ptr(&mut **self)
    }

    fn reserve(&mut self, len: usize) -> Result<(), compio::buf::ReserveError> {
        compio::buf::IoBufMut::reserve(&mut **self, len)
    }

    fn reserve_exact(&mut self, len: usize) -> Result<(), compio::buf::ReserveExactError> {
        compio::buf::IoBufMut::reserve_exact(&mut **self, len)
    }
}

impl std::fmt::Display for ReserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("buffer allocation failed")
    }
}

impl std::error::Error for ReserveError {}
