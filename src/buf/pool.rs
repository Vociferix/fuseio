use super::{ALIGN, Buf};

use std::cell::UnsafeCell;
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

    pub(super) fn checkout_owned(self) -> Buf {
        // SAFETY: The pool is behind an `Rc`, so every handle is on one
        //         thread, and this borrow of the `Vec` ends before the call
        //         returns. Nothing it calls can re-enter the pool.
        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = buf_opt.unwrap_or_else(|| AVec::new(ALIGN));
        Buf { buf, pool: self }
    }

    pub(super) fn checkout_with_capacity_owned(self, capacity: usize) -> Buf {
        // SAFETY: As in `checkout_owned`.
        let buf_opt = unsafe { (*self.pool.get()).pop() };
        let buf = match buf_opt {
            Some(mut buf) => {
                buf.reserve(capacity);
                buf
            }
            None => AVec::with_capacity(ALIGN, capacity),
        };

        Buf { buf, pool: self }
    }

    pub fn checkout(&self) -> Buf {
        self.clone().checkout_owned()
    }

    pub fn checkout_with_capacity(&self, capacity: usize) -> Buf {
        self.clone().checkout_with_capacity_owned(capacity)
    }

    pub(super) fn checkin(&self, mut buf: AVec<u8, ConstAlign<ALIGN>>) {
        if buf.capacity() > 0 {
            buf.clear();
            // SAFETY: As in `checkout_owned`; `buf` is owned, so pushing it
            //         cannot reach back into the pool either.
            unsafe {
                (*self.pool.get()).push(buf);
            }
        }
    }
}

impl Default for BufPool {
    fn default() -> Self {
        Self::new()
    }
}
