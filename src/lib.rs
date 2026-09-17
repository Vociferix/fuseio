mod async_arc;
mod async_rc;
mod buf;
mod builder;
mod dev_fuse;
mod error;
mod fs;
mod handle;
mod handshake;
mod ioctl;
mod options;
mod passthrough;
mod proto;
mod server;
mod types;

pub mod mount;

pub use fs::{BindFs, Fs, MountFs};

const MAX_WRITE_SIZE: usize = 16 * 1024 * 1024;

pub use buf::{Buf, BufPool, IntoIoBuf, Vectored};
pub use builder::Builder;
pub use error::Error;
pub use options::{MountOpt, ParseMountOptError};
pub use passthrough::{BackingId, PassthroughFd};

pub type Result<T> = std::result::Result<T, Error>;
