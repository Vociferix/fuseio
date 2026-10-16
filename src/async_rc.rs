use std::alloc::{Layout, alloc, dealloc};
use std::cell::Cell;
use std::mem::ManuallyDrop;
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
        // SAFETY: `Ctrl<T>` holds a `CtrlData`, so its layout is never zero
        //         sized, which is the one thing `alloc` rejects.
        let mem = unsafe { alloc(layout) as *mut Ctrl<T> };

        if mem.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        // SAFETY: `mem` is freshly allocated with exactly this type's layout
        //         and is not yet initialized, so there is no old value to drop.
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

        impl<T> Fut<T> {
            fn ctrl(&self) -> &Ctrl<T> {
                // SAFETY: `self.0` is a live allocation for as long as this
                //         future holds its reference count, which it does until
                //         `poll` takes the value and nulls the pointer. Nothing
                //         calls this afterwards: `poll` returns `Ready` in the
                //         same breath, and `drop` checks for null first.
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
                    // SAFETY: This future holds the last reference, so nothing
                    //         else can observe `value` being moved out. The
                    //         allocation is dropped below as a
                    //         `Ctrl<MaybeUninit<T>>` so that the moved-out value
                    //         is not dropped a second time.
                    let value = unsafe { std::ptr::read(&raw const (*self.0).value) };
                    let mem = self.0.cast_mut();
                    // SAFETY: The value was moved out just above, so the only
                    //         thing left to drop is the control block -- the
                    //         waker in particular. Dropping that field on its
                    //         own leaves the moved-out value alone, where
                    //         dropping the whole `Ctrl` would run its
                    //         destructor a second time.
                    unsafe {
                        std::ptr::drop_in_place(&raw mut (*mem).ctrl_data);
                    }
                    // SAFETY: The same layout the allocation was made with in
                    //         `new`, and the last reference is being given up
                    //         here, so nothing points at it afterwards.
                    unsafe {
                        dealloc(mem.cast(), Layout::new::<Ctrl<T>>());
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
        // SAFETY: Holding an `AsyncRc` means holding one of the allocation's
        //         reference counts, so it is live for at least as long as the
        //         borrow of `self`.
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
                // SAFETY: The count reached zero, so this is the last handle
                //         and no reference to the value outlives it. The layout
                //         matches the one `new` allocated with.
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::rc::Rc;
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

    /// Counts its own drops, so a value dropped twice -- or never -- shows up.
    struct Tracked(Rc<Cell<usize>>);

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn the_value_is_reachable_through_every_handle() {
        let rc = AsyncRc::new(41u32 + 1);
        let clone = rc.clone();

        assert_eq!(*rc, 42);
        assert_eq!(*clone, 42);
    }

    // The last handle owns the value, so taking it needs no waiting.
    #[test]
    fn the_only_handle_unwraps_without_waiting() {
        let waker = CountingWaker::new();
        let mut fut = std::pin::pin!(AsyncRc::new(7u8).unwrap());

        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(7)
        );
    }

    // This is the whole point of the type: shutdown waits for the workers still
    // holding a handle before taking the value back.
    #[test]
    fn unwrapping_waits_for_the_other_handles() {
        let waker = CountingWaker::new();
        let rc = AsyncRc::new(7u8);
        let clone = rc.clone();

        let mut fut = std::pin::pin!(rc.unwrap());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());
        assert_eq!(waker.count(), 0, "nothing has changed yet");

        drop(clone);

        assert_eq!(waker.count(), 1, "dropping the last clone must wake it");
        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(7)
        );
    }

    #[test]
    fn unwrapping_waits_for_all_of_several_handles() {
        let waker = CountingWaker::new();
        let rc = AsyncRc::new(7u8);
        let first = rc.clone();
        let second = rc.clone();

        let mut fut = std::pin::pin!(rc.unwrap());

        assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());

        drop(first);

        assert!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending(),
            "one handle is still out"
        );

        drop(second);

        assert_eq!(
            poll_once(fut.as_mut(), &Waker::from(waker.clone())),
            Poll::Ready(7)
        );
    }

    // The allocation holds the value and the control block together, and the
    // value is moved out rather than dropped in place, so unwrapping must not
    // run its destructor.
    #[test]
    fn unwrapping_hands_the_value_over_without_dropping_it() {
        let drops = Rc::new(Cell::new(0));
        let waker = CountingWaker::new();

        let taken = {
            let mut fut = std::pin::pin!(AsyncRc::new(Tracked(drops.clone())).unwrap());

            match poll_once(fut.as_mut(), &Waker::from(waker.clone())) {
                Poll::Ready(value) => value,
                Poll::Pending => panic!("the only handle should unwrap at once"),
            }
        };

        assert_eq!(drops.get(), 0, "the value was dropped on its way out");

        drop(taken);

        assert_eq!(drops.get(), 1, "the value should drop exactly once");
    }

    // Dropping the last handle without unwrapping has to drop the value, once.
    #[test]
    fn dropping_the_last_handle_drops_the_value_once() {
        let drops = Rc::new(Cell::new(0));

        {
            let rc = AsyncRc::new(Tracked(drops.clone()));
            let clone = rc.clone();

            drop(rc);
            assert_eq!(drops.get(), 0, "a handle is still out");

            drop(clone);
        }

        assert_eq!(drops.get(), 1);
    }

    // A future dropped before it completes gives its handle back, so the value
    // is not stranded.
    #[test]
    fn abandoning_the_unwrap_leaves_the_value_to_the_others() {
        let drops = Rc::new(Cell::new(0));
        let waker = CountingWaker::new();
        let rc = AsyncRc::new(Tracked(drops.clone()));
        let clone = rc.clone();

        {
            let mut fut = std::pin::pin!(rc.unwrap());
            assert!(poll_once(fut.as_mut(), &Waker::from(waker.clone())).is_pending());
        }

        assert_eq!(drops.get(), 0, "the remaining handle still owns it");

        drop(clone);

        assert_eq!(drops.get(), 1);
    }
}
