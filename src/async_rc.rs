use std::cell::Cell;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

pub struct AsyncRc<T> {
    inner: Rc<Inner<T>>,
}

struct Inner<T> {
    value: T,
    waker: Cell<Option<Waker>>,
}

impl<T> AsyncRc<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(Inner {
                value,
                waker: Cell::new(None),
            }),
        }
    }

    pub async fn unwrap(this: Self) -> T {
        struct Fut<T>(Option<Rc<Inner<T>>>);

        impl<T> Future for Fut<T> {
            type Output = T;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
                let inner = unsafe { self.0.take().unwrap_unchecked() };
                match Rc::try_unwrap(inner) {
                    Ok(Inner { value, .. }) => Poll::Ready(value),
                    Err(inner) => {
                        inner.waker.set(Some(cx.waker().clone()));
                        self.0 = Some(inner);
                        Poll::Pending
                    }
                }
            }
        }

        let this = ManuallyDrop::new(this);
        let inner = unsafe { std::ptr::read(&this.inner) };

        Fut(Some(inner)).await
    }
}

impl<T> std::fmt::Debug for AsyncRc<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner.value, f)
    }
}

impl<T> Clone for AsyncRc<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std::ops::Deref for AsyncRc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner.value
    }
}

impl<T> Drop for AsyncRc<T> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 2
            && let Some(waker) = self.inner.waker.take()
        {
            waker.wake();
        }
    }
}
