use std::cell::UnsafeCell;
use std::rc::Rc;
use std::marker::PhantomData;

use aligned_vec::{AVec, ConstAlign};

const ALIGN: usize = std::mem::align_of::<u64>();

#[derive(Debug)]
pub struct BufPool {
    pool: Rc<UnsafeCell<Vec<AVec<u8, ConstAlign<ALIGN>>>>>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Buf<T> {
    buf: AVec<u8, ConstAlign<ALIGN>>,
    _phantom: PhantomData<AVec<T, ConstAlign<ALIGN>>>,
}

#[derive(Debug, Clone)]
pub struct BufGuard<T> {
    buf: Buf<T>,
    pool: BufPool,
}

impl BufPool {
    pub fn new() -> Self {
        Self {
            pool: Rc::new(UnsafeCell::new(Vec::new())),
        }
    }

    pub fn checkout<T: bytemuck::Pod>(&self) -> BufGuard<T> {
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

    pub fn checkout_with_capacity<T: bytemuck::Pod>(&self, capacity: usize) -> BufGuard<T> {
        assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());
        let buf_opt: Option<Buf<T>> = unsafe { (*self.pool.get()).pop() };
        let buf = match buf_opt {
            Some(mut buf) => {
                Buf::reserve(&mut buf, capacity);
                buf
            },
            None => Buf::with_capacity(std::mem::align_of::<u64>(), capacity),
        };
        BufGuard {
            buf,
            pool: self.clone(),
        }
    }

    fn checkin<T>(&self, mut buf: Buf<T>) {
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
        Self { pool: self.pool.clone() }
    }
}

impl<T> Drop for BufGuard<T> {
    fn drop(&mut self) {
        let buf = std::mem::take(&mut self.buf);
        self.pool.checkin(buf);
    }
}

impl<T> std::ops::Deref for BufGuard<T> {
    type Target = Buf<T>;

    fn deref(&self) -> &Buf<T> {
        &self.buf
    }
}

impl<T> std::ops::DerefMut for BufGuard<T> {
    fn deref_mut(&mut self) -> &mut Buf<T> {
        &mut self.buf
    }
}

impl<T: bytemuck::Pod> Buf<T> {
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
            buf: Avec::with_capacity(ALIGN, capacity * std::mem::size_of::<T>()),
            _phantom: PhantomData,
        }
    }
}

impl<T: bytemuck::Pod + std::fmt::Debug> std::fmt::Debug for Buf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}
