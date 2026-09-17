use std::alloc::{Layout, alloc, dealloc};
use std::cell::Cell;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

pub struct AsyncRc<T> {
    ctrl: *const Ctrl<T>,
}

struct Ctrl<T> {
    ctrl_data: CtrlData,
    value: T,
}

struct CtrlData {
    ref_cnt: Cell<usize>,
    waker: Cell<Option<Waker>>,
}

impl<T> AsyncRc<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<Ctrl<T>>();
        let mem = unsafe { alloc(layout) as *mut Ctrl<T> };

        unsafe {
            std::ptr::write(
                mem,
                Ctrl {
                    ctrl_data: CtrlData {
                        ref_cnt: Cell::new(1),
                        waker: Cell::new(None),
                    },
                    value,
                },
            );
        }

        Self { ctrl: mem }
    }

    pub async fn unwrap(self) -> T {
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

                drop(AsyncRc { ctrl: self.0 });
            }
        }

        impl<T> Future for Fut<T> {
            type Output = T;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.ctrl().ctrl_data.ref_cnt.get() == 1 {
                    let value = unsafe { std::ptr::read(&raw const (*self.0).value) };
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
                self.ctrl().ctrl_data.waker.set(Some(cx.waker().clone()));
                Poll::Pending
            }
        }

        let this = ManuallyDrop::new(self);

        Fut(this.ctrl).await
    }

    fn ctrl(&self) -> &Ctrl<T> {
        unsafe { &*self.ctrl }
    }
}

impl<T> Drop for AsyncRc<T> {
    fn drop(&mut self) {
        let new_ref_cnt = self.ctrl().ctrl_data.ref_cnt.get() - 1;
        self.ctrl().ctrl_data.ref_cnt.set(new_ref_cnt);
        match new_ref_cnt {
            0 => {
                let ctrl = self.ctrl as *mut Ctrl<T>;
                unsafe {
                    std::ptr::drop_in_place(ctrl);
                    dealloc(ctrl as *mut u8, Layout::new::<Ctrl<T>>());
                }
            }
            1 => {
                if let Some(waker) = self.ctrl().ctrl_data.waker.take() {
                    waker.wake();
                }
            }
            _ => {}
        }
    }
}

impl<T> Clone for AsyncRc<T> {
    fn clone(&self) -> Self {
        self.ctrl().ctrl_data.ref_cnt.update(|ref_cnt| ref_cnt + 1);
        Self { ctrl: self.ctrl }
    }
}

impl<T> std::ops::Deref for AsyncRc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.ctrl().value
    }
}

impl<T> std::fmt::Debug for AsyncRc<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
