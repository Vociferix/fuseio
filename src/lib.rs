mod async_rc;
mod buf_pool;
mod builder;
mod channel;
mod conf;
mod error;
mod fs;
mod handle;
mod layout;
mod options;
mod passthrough;
mod serve;

pub mod mount;
pub mod request;

pub use fs::Filesystem;

const MAX_WRITE_SIZE: usize = 16 * 1024 * 1024;

pub use builder::Builder;
pub use conf::{FsConfig, InitFlags, KernelConfig, Version};
pub use error::Error;
pub use handle::MountHandle;
pub use options::{MountOpt, ParseMountOptError};
pub use passthrough::{BackingId, PassthroughFd};

pub type Result<T> = std::result::Result<T, Error>;

pub async fn mount<F, P, I>(fs: F, mountpoint: P, options: I) -> std::io::Result<MountHandle>
where
    F: Filesystem,
    P: AsRef<std::path::Path>,
    I: IntoIterator<Item = MountOpt>,
{
    Builder::new().options(options).mount(fs, mountpoint).await
}
