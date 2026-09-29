#![allow(async_fn_in_trait)]

mod async_rc;
mod builder;
mod cancel_token;
mod context;
mod error;
mod handle;
mod handshake;
mod notify_error;
mod options;
mod passthrough;
mod proto;
mod req;
mod server;
mod types;

pub mod buf;
pub mod conn;
pub mod fs;
pub mod mount;

pub use builder::Builder;
pub use error::Error;
pub use options::{MountOpt, ParseMountOptError};

pub type Result<T> = std::result::Result<T, Error>;

fn select_biased<A, B>(a: A, b: B) -> impl Future<Output = either::Either<A::Output, B::Output>>
where
    A: Future,
    B: Future,
{
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct SelectBiased<A, B> {
        a: A,
        b: B,
    }

    impl<A, B> Future for SelectBiased<A, B>
    where
        A: Future,
        B: Future,
    {
        type Output = either::Either<A::Output, B::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { Pin::get_unchecked_mut(self) };

            let a = unsafe { Pin::new_unchecked(&mut this.a) };
            let b = unsafe { Pin::new_unchecked(&mut this.b) };

            if let Poll::Ready(a) = a.poll(cx) {
                return Poll::Ready(either::Left(a));
            }

            if let Poll::Ready(b) = b.poll(cx) {
                return Poll::Ready(either::Right(b));
            }

            Poll::Pending
        }
    }

    SelectBiased { a, b }
}

fn mount_blocking<M, F>(
    builder: Builder<M>,
    fs: F,
    mountpoint: impl AsRef<std::path::Path>,
) -> std::io::Result<()>
where
    M: mount::Mount,
    M::Unmount: Send,
    M::SharedConn: Send,
    F: fs::MountFs<M::Conn>,
    F::Fs: Clone + Send + 'static,
{
    // TODO: Add a way to customize affinity - currently not
    // possible to isolate mounts to a particular subset of
    // cores or disable affinity. The former would be a problem
    // for multiple mounts that the user wants to allocate to
    // specific cores, but would currently just use the same
    // first `n` cores. However, users can use the `Builder::mount`
    // method and manually create their own runtimes, so the
    // capability isn't completely blocked.

    let affinity = (builder.workers > 1).then_some(0usize);

    let runtime = compio::runtime::Runtime::builder()
        .thread_affinity(affinity.into_iter().collect())
        .build()?;

    let mut iter = runtime.block_on(builder.mount(fs, mountpoint))?;

    let Some(handle) = iter.next() else {
        return Ok(());
    };

    let threads: Vec<_> = iter
        .map(|handle| {
            std::thread::spawn(move || {
                compio::runtime::Runtime::builder()
                    .thread_affinity(std::iter::once(handle.id).collect())
                    .build()?
                    .block_on(handle.bind_and_serve())
            })
        })
        .collect();

    let mut res = runtime.block_on(handle.bind_and_serve());

    for thread in threads {
        if let Err(err) = thread.join().unwrap()
            && res.is_ok()
        {
            res = Err(err);
        }
    }

    res
}
