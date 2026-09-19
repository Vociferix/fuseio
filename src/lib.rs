mod async_arc;
mod async_rc;
mod buf;
mod builder;
mod context;
mod dev_fuse;
mod error;
mod handle;
mod handshake;
mod ioctl;
mod options;
mod passthrough;
mod proto;
mod req;
mod server;
mod types;

pub mod fs;
pub mod mount;

const MAX_WRITE_SIZE: usize = 16 * 1024 * 1024;

pub use buf::{Buf, BufPool, IntoIoBuf, Vectored};
pub use builder::Builder;
pub use error::Error;
pub use options::{MountOpt, ParseMountOptError};

pub type Result<T> = std::result::Result<T, Error>;
