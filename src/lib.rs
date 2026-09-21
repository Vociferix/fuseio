mod async_arc;
mod async_rc;
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

pub mod buf;
pub mod fs;
pub mod mount;

pub use builder::Builder;
pub use error::Error;
pub use options::{MountOpt, ParseMountOptError};

pub type Result<T> = std::result::Result<T, Error>;

#[doc(hidden)]
pub mod __internal {
    pub use nix;
}
