use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use slab::Slab;

#[derive(Clone)]
pub struct CancelToken {
    inner: Rc<Inner>,
}

struct Inner {
    cancelled: Cell<bool>,
    wakers: RefCell<Slab<Waker>>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                cancelled: Cell::new(false),
                wakers: RefCell::new(Slab::new()),
            }),
        }
    }

    pub(crate) fn try_reset(&self) -> bool {
        if Rc::strong_count(&self.inner) == 1 {
            self.inner.cancelled.set(false);
            self.inner.wakers.borrow_mut().clear();
            true
        } else {
            false
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.get()
    }

    pub fn cancel(&self) {
        if !self.inner.cancelled.replace(true) {
            let mut wakers = std::mem::take(&mut *self.inner.wakers.borrow_mut());
            wakers.drain().for_each(Waker::wake);
            *self.inner.wakers.borrow_mut() = wakers;
        }
    }

    pub fn cancelled(&self) -> impl Future<Output = ()> {
        struct Cancelled<'a> {
            token: &'a Inner,
            id: Option<usize>,
        }

        impl Future for Cancelled<'_> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.token.cancelled.get() {
                    self.id = None;
                    return Poll::Ready(());
                }

                let mut wakers = self.token.wakers.borrow_mut();

                if let Some(id) = self.id {
                    let waker = unsafe { wakers.get_mut(id).unwrap_unchecked() };
                    if !waker.will_wake(cx.waker()) {
                        *waker = cx.waker().clone();
                    }
                } else {
                    self.id = Some(wakers.insert(cx.waker().clone()));
                }

                Poll::Pending
            }
        }

        impl Drop for Cancelled<'_> {
            fn drop(&mut self) {
                if let Some(id) = self.id {
                    self.token.wakers.borrow_mut().try_remove(id);
                }
            }
        }

        Cancelled {
            token: &self.inner,
            id: None,
        }
    }

    pub async fn run_until_cancelled<F>(&self, future: F) -> Option<F::Output>
    where
        F: Future,
    {
        if let either::Left(output) = crate::select_biased(future, self.cancelled()).await {
            Some(output)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::poll_fn;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;

    #[derive(Default)]
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl CountingWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn count(&self) -> usize {
            self.wakes.load(Ordering::Relaxed)
        }
    }

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn poll_once<F: Future>(fut: Pin<&mut F>, waker: &Waker) -> Poll<F::Output> {
        fut.poll(&mut Context::from_waker(waker))
    }

    #[test]
    fn starts_uncancelled() {
        let token = CancelToken::new();

        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_sets_the_flag_for_every_clone() {
        let token = CancelToken::new();
        let clone = token.clone();

        clone.cancel();

        assert!(token.is_cancelled());
        assert!(clone.is_cancelled());
    }

    #[test]
    fn cancel_wakes_a_registered_waiter() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());
        assert_eq!(waker.count(), 0);

        token.cancel();

        assert_eq!(waker.count(), 1);
        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_ready());
    }

    #[test]
    fn cancel_wakes_every_waiter() {
        let token = CancelToken::new();
        let first = CountingWaker::new();
        let second = CountingWaker::new();
        let mut first_fut = std::pin::pin!(token.cancelled());
        let mut second_fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(first_fut.as_mut(), &Waker::from(first.clone())).is_pending());
        assert!(poll_once(second_fut.as_mut(), &Waker::from(second.clone())).is_pending());

        token.cancel();

        assert_eq!(first.count(), 1);
        assert_eq!(second.count(), 1);
    }

    #[test]
    fn cancelling_twice_wakes_once() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());

        token.cancel();
        token.cancel();

        assert_eq!(waker.count(), 1);
    }

    #[test]
    fn dropping_a_waiter_leaves_the_others_registered() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let mut kept = std::pin::pin!(token.cancelled());

        {
            let mut dropped = std::pin::pin!(token.cancelled());
            assert!(poll_once(dropped.as_mut(), &Waker::from(waker.clone())).is_pending());
            assert!(poll_once(kept.as_mut(), &Waker::from(waker.clone())).is_pending());
        }

        token.cancel();

        assert_eq!(waker.count(), 1);
        assert!(poll_once(kept.as_mut(), &Waker::from(waker.clone())).is_ready());
    }

    #[test]
    fn repolling_replaces_a_stale_waker() {
        let token = CancelToken::new();
        let first = CountingWaker::new();
        let second = CountingWaker::new();
        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(first.clone())).is_pending());
        assert!(poll_once(fut.as_mut(), &Waker::from(second.clone())).is_pending());

        token.cancel();

        assert_eq!(first.count(), 0);
        assert_eq!(second.count(), 1);
    }

    #[test]
    fn waiting_on_a_cancelled_token_is_ready() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();

        token.cancel();

        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_ready());
        assert_eq!(waker.count(), 0);
    }

    #[test]
    fn dropping_a_registered_waiter_after_cancelling_is_harmless() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();

        {
            let mut fut = std::pin::pin!(token.cancelled());
            assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());
            token.cancel();
        }

        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_ready());
    }

    #[test]
    fn run_until_cancelled_returns_the_output() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let mut fut = std::pin::pin!(token.run_until_cancelled(std::future::ready(7)));

        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(Some(7))
        );
    }

    #[test]
    fn run_until_cancelled_gives_up_when_cancelled() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let mut fut = std::pin::pin!(token.run_until_cancelled(std::future::pending::<u8>()));

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());

        token.cancel();

        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(None)
        );
    }

    #[test]
    fn run_until_cancelled_prefers_an_output_ready_with_the_cancellation() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();
        let inner = poll_fn(|_| {
            if token.is_cancelled() {
                Poll::Ready(7)
            } else {
                Poll::Pending
            }
        });
        let mut fut = std::pin::pin!(token.run_until_cancelled(inner));

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());

        token.cancel();

        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(Some(7))
        );
    }

    #[test]
    fn try_reset_fails_while_a_clone_is_alive() {
        let token = CancelToken::new();
        let clone = token.clone();

        token.cancel();

        assert!(!token.try_reset());
        assert!(token.is_cancelled());

        drop(clone);

        assert!(token.try_reset());
        assert!(!token.is_cancelled());
    }

    #[test]
    fn a_reset_token_can_be_cancelled_again() {
        let token = CancelToken::new();
        let waker = CountingWaker::new();

        {
            let mut fut = std::pin::pin!(token.cancelled());
            assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());
            token.cancel();
        }

        assert!(token.try_reset());

        let second = CountingWaker::new();
        let mut fut = std::pin::pin!(token.cancelled());

        assert!(poll_once(fut.as_mut(), &Waker::from(second.clone())).is_pending());

        token.cancel();

        assert_eq!(second.count(), 1);
        assert!(poll_once(fut.as_mut(), &Waker::from(second.clone())).is_ready());
    }
}
