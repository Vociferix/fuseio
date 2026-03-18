use super::{ALIGN, Buf};

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::rc::Rc;

use aligned_vec::{AVec, ConstAlign};

#[derive(Debug, Clone)]
pub struct BufPool {
    pool: Rc<UnsafeCell<Vec<AVec<u8, ConstAlign<ALIGN>>>>>,
}

impl BufPool {
    pub fn new() -> Self {
        Self {
            pool: Rc::new(UnsafeCell::new(Vec::new())),
        }
    }

    pub(super) fn checkout_owned<T>(self) -> Buf<T> {
        assert!(std::mem::align_of::<T>() <= ALIGN);

        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = buf_opt.unwrap_or_else(|| AVec::new(ALIGN));
        Buf {
            buf,
            pool: self,
            _phantom: PhantomData,
        }
    }

    pub(super) fn checkout_with_capacity_owned<T>(self, capacity: usize) -> Buf<T> {
        assert!(std::mem::align_of::<T>() <= ALIGN);

        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = match buf_opt {
            Some(mut buf) => {
                buf.reserve(capacity * std::mem::size_of::<T>());
                buf
            }
            None => AVec::with_capacity(ALIGN, capacity * std::mem::size_of::<T>()),
        };

        Buf {
            buf,
            pool: self,
            _phantom: PhantomData,
        }
    }

    pub fn checkout<T>(&self) -> Buf<T> {
        self.clone().checkout_owned()
    }

    pub fn checkout_with_capacity<T>(&self, capacity: usize) -> Buf<T> {
        self.clone().checkout_with_capacity_owned(capacity)
    }

    pub(super) fn checkin(&self, mut buf: AVec<u8, ConstAlign<ALIGN>>) {
        if buf.capacity() > 0 {
            buf.clear();
            unsafe {
                (*self.pool.get()).push(buf);
            }
        }
    }
}
