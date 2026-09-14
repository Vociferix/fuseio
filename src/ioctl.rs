#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BackingMap {
    pub fd: i32,
    pub flags: u32,
    pub _unused: u64,
}

nix::ioctl_read!(clone_fd, 299, 1, i32);
nix::ioctl_write_ptr!(passthrough_open, 299, 1, BackingMap);
nix::ioctl_write_ptr!(passthrough_close, 299, 2, u32);
