mod async_arc;
mod async_rc;
mod builder;
mod cancel_token;
mod context;
mod dev_fuse;
mod error;
mod handle;
mod handshake;
mod ioctl;
mod notify_error;
mod options;
mod passthrough;
mod proto;
mod req;
mod server;
mod types;

pub mod buf;
pub mod fs;
pub mod mount;

pub use builder::Builder;
pub use error::Error;
pub use notify_error::NotifyError;
pub use options::{MountOpt, ParseMountOptError};

pub type Result<T> = std::result::Result<T, Error>;

#[doc(hidden)]
pub mod __internal {
    pub use nix;
}

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
