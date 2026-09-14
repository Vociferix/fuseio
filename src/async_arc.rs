use std::alloc::{Layout, alloc, dealloc};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use atomic_waker::AtomicWaker;
use crossbeam_utils::CachePadded;

pub struct AsyncArc<T> {
    ctrl: *const Ctrl<T>,
}

struct Ctrl<T> {
    ctrl_data: CachePadded<CtrlData>,
    value: CachePadded<T>,
}

struct CtrlData {
    ref_cnt: AtomicUsize,
    waker: AtomicWaker,
}

impl<T> AsyncArc<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<Ctrl<T>>();
        let mem = unsafe { alloc(layout) as *mut Ctrl<T> };

        unsafe {
            std::ptr::write(
                mem,
                Ctrl {
                    ctrl_data: CachePadded::new(CtrlData {
                        ref_cnt: AtomicUsize::new(1),
                        waker: AtomicWaker::new(),
                    }),
                    value: CachePadded::new(value),
                },
            );
        }

        Self { ctrl: mem }
    }

    pub async fn unwrap(this: Self) -> T {
        struct Fut<T>(*const Ctrl<T>);

        impl<T> Unpin for Fut<T> {}

        unsafe impl<T> Send for Fut<T> where T: Send + Sync {}

        impl<T> Fut<T> {
            fn ctrl(&self) -> &Ctrl<T> {
                unsafe { &*self.0 }
            }
        }

        impl<T> Drop for Fut<T> {
            fn drop(&mut self) {
                if self.0.is_null() {
                    return;
                }

                drop(AsyncArc { ctrl: self.0 });
            }
        }

        impl<T> Future for Fut<T> {
            type Output = T;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                self.ctrl().ctrl_data.waker.register(cx.waker());
                if self.ctrl().ctrl_data.ref_cnt.load(Ordering::Acquire) == 1 {
                    let value = unsafe { std::ptr::read(&raw const (*self.0).value) }.into_inner();
                    let mem = self.0 as *mut Ctrl<T> as *mut Ctrl<MaybeUninit<T>>;
                    unsafe {
                        std::ptr::drop_in_place(mem);
                    }
                    unsafe {
                        dealloc(mem as *mut u8, Layout::new::<Ctrl<T>>());
                    }
                    self.0 = std::ptr::null();
                    return Poll::Ready(value);
                }
                Poll::Pending
            }
        }

        let this = ManuallyDrop::new(this);

        Fut(this.ctrl).await
    }

    fn ctrl(&self) -> &Ctrl<T> {
        unsafe { &*self.ctrl }
    }
}

unsafe impl<T> Send for AsyncArc<T> where T: Send + Sync {}

unsafe impl<T> Sync for AsyncArc<T> where T: Send + Sync {}

impl<T> Drop for AsyncArc<T> {
    fn drop(&mut self) {
        match self
            .ctrl()
            .ctrl_data
            .ref_cnt
            .fetch_sub(1, Ordering::Release)
        {
            1 => {
                let ctrl = self.ctrl as *mut Ctrl<T>;
                unsafe {
                    std::ptr::drop_in_place(ctrl);
                    dealloc(ctrl as *mut u8, Layout::new::<Ctrl<T>>());
                }
            }
            2 => {
                self.ctrl().ctrl_data.waker.wake();
            }
            _ => {}
        }
    }
}

impl<T> Clone for AsyncArc<T> {
    fn clone(&self) -> Self {
        self.ctrl()
            .ctrl_data
            .ref_cnt
            .fetch_add(1, Ordering::Acquire);
        Self { ctrl: self.ctrl }
    }
}

impl<T> std::ops::Deref for AsyncArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.ctrl().value
    }
}

impl<T> std::fmt::Debug for AsyncArc<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
