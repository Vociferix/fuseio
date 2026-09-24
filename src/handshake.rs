use crate::MountOpt;
use crate::async_arc::AsyncArc;
use crate::dev_fuse::DevFuse;
use crate::fs::{BindFs, MountFs};
use crate::proto::request::Opcode;
use crate::types::{FsCaps, KernelCaps, KernelInitFlags, ReplyInitFlags, Version};

use std::io::{ErrorKind, Result};
use std::mem::size_of;
use std::os::fd::AsFd;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct KernelConfig {
    ver: Version,
    max_readahead: usize,
    caps: KernelCaps,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    caps: FsCaps,
    max_readahead: u32,
    max_background: u16,
    congestion_threshold: u16,
    max_write: u32,
    time_gran: u32,
    ignore_interrupts: bool,
}

pub struct Init<F> {
    pub fs: F,
    pub ver: Version,
    pub flags: ReplyInitFlags,
    pub max_readahead: usize,
    pub config: Config,
}

#[repr(C)]
struct ReqHdr {
    len: u32,
    opcode: Opcode,
    unique: u64,
    _nodeid: u64,
    _uid: u32,
    _gid: u32,
    _pid: u32,
    _unused: u32,
}

#[repr(C)]
struct InitReqRaw {
    hdr: ReqHdr,
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags0: u32,
    flags1: u32,
    _unused: u32,
}

#[repr(C)]
#[derive(Default)]
struct RespHdr {
    len: u32,
    err: i32,
    unique: u64,
}

#[repr(C)]
#[derive(Default)]
struct InitRespRaw {
    hdr: RespHdr,
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags1: u32,
    max_background: u16,
    congestion_threshold: u16,
    max_write: u32,
    time_gran: u32,
    max_pages: u16,
    map_alignment: u16,
    flags2: u32,
    max_stack_depth: u32,
    request_timeout: u16,
    _unused: [u16; 11],
}

// These are read from and written to the device as raw bytes, so they must match
// the kernel's layouts exactly and contain no padding.
const _: () = assert!(size_of::<ReqHdr>() == 40);
const _: () = assert!(size_of::<InitReqRaw>() == 64);
const _: () = assert!(size_of::<RespHdr>() == 16);
const _: () = assert!(size_of::<InitRespRaw>() == 80);

pub const MAJOR_VER: u32 = 7;
pub const MINOR_VER: u32 = 45;

/// `FUSE_MIN_READ_BUFFER`: the kernel rejects device reads into smaller buffers.
const MIN_READ_BUFFER: usize = 8192;

/// The kernel raises anything smaller to this, so `max_write` is clamped to it.
const MIN_MAX_WRITE: u32 = 4096;

/// The kernel ignores a `time_gran` above one second.
const MAX_TIME_GRAN: u32 = 1_000_000_000;

/// `fuse_init_out` body sizes expected by kernels older than 7.5 and 7.23.
const COMPAT_INIT_OUT_SIZE: usize = 8;
const COMPAT_22_INIT_OUT_SIZE: usize = 24;

impl KernelConfig {
    pub fn version(&self) -> Version {
        self.ver
    }

    pub fn max_readahead(&self) -> Option<usize> {
        (self.ver.1 >= 6).then_some(self.max_readahead)
    }

    pub fn caps(&self) -> KernelCaps {
        self.caps
    }

    pub fn to_config(&self) -> Config {
        Config {
            caps: FsCaps::defaults(self.caps),
            max_readahead: self.max_readahead as u32,
            max_background: 64,
            // Zero selects 3/4 of `max_background`, as libfuse does.
            congestion_threshold: 0,
            max_write: crate::server::MAX_WRITE_SIZE as u32,
            time_gran: 1,
            ignore_interrupts: false,
        }
    }
}

impl Config {
    /// Returns the features the filesystem enabled.
    pub fn caps(&self) -> FsCaps {
        self.caps
    }

    /// Returns the maximum readahead, in bytes.
    pub fn max_readahead(&self) -> usize {
        self.max_readahead as usize
    }

    /// Returns how many background requests the kernel may have in flight, or
    /// zero if the kernel's own default applies.
    pub fn max_background(&self) -> u16 {
        self.max_background
    }

    /// Returns the number of background requests at which the kernel considers
    /// the filesystem congested, or [`None`] if it follows
    /// [`max_background`](Self::max_background).
    pub fn congestion_threshold(&self) -> Option<u16> {
        (self.congestion_threshold != 0).then_some(self.congestion_threshold)
    }

    /// Returns the largest write request the filesystem accepts, in bytes.
    pub fn max_write(&self) -> usize {
        self.max_write as usize
    }

    /// Returns the granularity of the filesystem's timestamps.
    pub fn time_gran(&self) -> Duration {
        Duration::from_nanos(self.time_gran.into())
    }

    pub fn ignore_interrupts(&self) -> bool {
        self.ignore_interrupts
    }

    /// Sets the features the filesystem enables, replacing the defaults.
    ///
    /// Enabling a feature the kernel didn't offer fails the mount, so mask the
    /// wanted features with
    /// [`FsCaps::supported`](crate::types::FsCaps::supported) when the
    /// filesystem should run against kernels that may lack them.
    pub fn with_caps(mut self, caps: FsCaps) -> Self {
        self.caps = caps;
        self
    }

    /// Enables features in addition to those already set.
    pub fn enable(mut self, caps: FsCaps) -> Self {
        self.caps = self.caps.union(caps);
        self
    }

    /// Disables features, including any enabled by default.
    pub fn disable(mut self, caps: FsCaps) -> Self {
        self.caps = self.caps.difference(caps);
        self
    }

    /// Sets the maximum readahead, in bytes.
    ///
    /// The kernel uses the smaller of this and its own value, so this can only
    /// lower readahead below what
    /// [`KernelConfig::max_readahead`] reported.
    pub fn with_max_readahead(mut self, max_readahead: usize) -> Self {
        self.max_readahead = max_readahead.try_into().unwrap_or(u32::MAX);
        self
    }

    /// Sets how many background requests the kernel may have in flight.
    ///
    /// Zero leaves the kernel's own default in place.
    pub fn with_max_background(mut self, max_background: u16) -> Self {
        self.max_background = max_background;
        self
    }

    /// Sets the number of background requests at which the kernel considers the
    /// filesystem congested.
    ///
    /// [`None`] selects three quarters of
    /// [`with_max_background`](Self::with_max_background), as libfuse does. The
    /// kernel caps this at the background limit.
    pub fn with_congestion_threshold(mut self, threshold: impl Into<Option<u16>>) -> Self {
        self.congestion_threshold = threshold.into().unwrap_or(0);
        self
    }

    /// Sets the largest write request the filesystem accepts, in bytes.
    ///
    /// Clamped to at least 4 KiB, which the kernel enforces anyway, and to
    /// [`u32::MAX`]. Each worker allocates a receive buffer of this size plus
    /// room for a request header.
    pub fn with_max_write(mut self, max_write: usize) -> Self {
        let max_write = u32::try_from(max_write).unwrap_or(u32::MAX);
        self.max_write = max_write.max(MIN_MAX_WRITE);
        self
    }

    /// Sets the granularity of the filesystem's timestamps.
    ///
    /// Clamped to between one nanosecond and one second. The kernel rounds
    /// timestamps it assigns down to a multiple of this.
    pub fn with_time_gran(mut self, time_gran: Duration) -> Self {
        self.time_gran = time_gran
            .as_nanos()
            .clamp(1, u128::from(MAX_TIME_GRAN))
            .try_into()
            .unwrap_or(MAX_TIME_GRAN);
        self
    }

    pub fn with_ignore_interrupts(mut self, ignore: bool) -> Self {
        self.ignore_interrupts = ignore;
        self
    }
}

pub async fn handshake<F: MountFs>(
    fs: F,
    dev: AsyncArc<DevFuse>,
    opts: &[MountOpt],
) -> Result<Init<F::Fs>> {
    let msg = loop {
        let msg = read_init_req(dev.clone()).await?;

        // TODO: every 7.x minor is accepted, but several compat paths for old
        // minors are wrong: the <7.9 entry/attr reply sizes (`InodeAttrsCompat`
        // lacks nlink/uid/gid/rdev), CREATE before 7.12 (`fuse_open_in`
        // layout), RELEASE before 7.8 (16-byte body) and READ before 7.9.
        // Current kernels are all ≥ 7.12 (Linux ≥ 2.6.31, macOS 7.19, FreeBSD
        // 12.1+ 7.28), so requiring 7.12 would let these paths be deleted.
        // `max_readahead` and `flags` only exist from 7.6.
        let min_len = size_of::<ReqHdr>() + if msg.minor >= 6 { 16 } else { 8 };

        if msg.hdr.opcode != Opcode::INIT
            || (msg.hdr.len as usize) < min_len
            || msg.major < MAJOR_VER
        {
            let _ = write_init_err(dev.clone(), msg.hdr.unique, crate::Error::EPROTO).await;

            if msg.hdr.opcode != Opcode::INIT {
                log::error!(
                    "expected INIT opcode from kernel, received {}",
                    msg.hdr.opcode
                );
            } else if (msg.hdr.len as usize) < min_len {
                log::error!(
                    "invalid INIT request received from kernel: message length too small ({})",
                    msg.hdr.len
                );
            } else {
                log::error!(
                    "kernel FUSE version unsupported: {}.{} (minimum supported {}.0)",
                    msg.major,
                    msg.minor,
                    MAJOR_VER
                );
            }

            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "invalid initialization request from kernel",
            ));
        }

        // Reply with our version and wait for the kernel to retry with 7.x.
        if msg.major > MAJOR_VER {
            write_init_resp(
                dev.clone(),
                InitRespRaw {
                    hdr: RespHdr {
                        len: const { size_of::<InitRespRaw>() as u32 },
                        err: 0,
                        unique: msg.hdr.unique,
                    },
                    major: MAJOR_VER,
                    minor: MINOR_VER,
                    ..InitRespRaw::default()
                },
            )
            .await?;
            continue;
        }

        break msg;
    };

    let unique = msg.hdr.unique;

    let (max_readahead, kflags) = if msg.minor >= 6 {
        (
            msg.max_readahead,
            KernelInitFlags::from_wire(msg.flags0, msg.flags1),
        )
    } else {
        (0, KernelInitFlags::empty())
    };

    let kconf = KernelConfig {
        ver: Version(msg.major, msg.minor),
        max_readahead: max_readahead as usize,
        caps: KernelCaps::new(kflags, msg.minor),
    };

    let fs = match fs.mount(kconf, opts).await {
        Ok(fs) => fs,
        Err(err) => {
            log::error!("failed to initialize filesystem: {err}");
            let _ = write_init_err(dev.clone(), unique, err).await;
            return Err(err.into());
        }
    };

    let config = fs.config();

    // The kernel uses the smaller of its offer and the reply, so userspace can
    // only lower readahead; clamp so `Init` reports the value in effect (libfuse
    // does the same).
    let max_readahead = config.max_readahead.min(max_readahead);

    let unsupported = config.caps.unsupported(kconf.caps);
    if !unsupported.is_empty() {
        let _ = write_init_err(dev.clone(), unique, crate::Error::EPROTO).await;

        log::error!("filsystem enabled capabilities the kernel didn't offer: {unsupported:?}");

        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("filesystem enabled capabilities the kernel didn't offer: {unsupported:?}"),
        ));
    }

    let flags = ReplyInitFlags::negotiate(kflags, config.caps, false);
    let (flags1, flags2) = flags.to_wire();

    let mut resp = InitRespRaw {
        hdr: RespHdr {
            len: 0,
            err: 0,
            unique,
        },
        major: MAJOR_VER,
        minor: MINOR_VER,
        max_readahead,
        flags1,
        max_write: config.max_write,
        // Only read by the kernel when the reply sets MAP_ALIGNMENT, which this
        // crate never negotiates (DAX is virtio-fs only), so it stays zero.
        flags2,
        ..InitRespRaw::default()
    };

    if msg.minor >= 13 {
        resp.max_background = config.max_background;
        resp.congestion_threshold = match config.congestion_threshold {
            0 => (u32::from(config.max_background) * 3 / 4) as u16,
            threshold => threshold.min(config.max_background),
        };
    }

    if msg.minor >= 23 {
        resp.time_gran = config.time_gran;
    }

    if flags.contains(ReplyInitFlags::MAX_PAGES) {
        let page_size = page_size::get() as u32;
        resp.max_pages = ((config.max_write.max(1) - 1) / page_size + 1)
            .try_into()
            .unwrap_or(u16::MAX);
    }

    if flags.contains(ReplyInitFlags::PASSTHROUGH) {
        // Counts the FUSE layer itself; 1 means backing files may not be on a
        // stacked filesystem (libfuse's default, FUSE_BACKING_STACKED_UNDER).
        resp.max_stack_depth = 1;
    }

    let body_len = if msg.minor < 5 {
        COMPAT_INIT_OUT_SIZE
    } else if msg.minor < 23 {
        COMPAT_22_INIT_OUT_SIZE
    } else {
        size_of::<InitRespRaw>() - size_of::<RespHdr>()
    };
    resp.hdr.len = (size_of::<RespHdr>() + body_len) as u32;

    write_init_resp(dev, resp).await?;

    let ver = Version(MAJOR_VER, msg.minor.min(MINOR_VER));

    log::info!("FUSE initialized for protocol version {ver}");

    Ok(Init {
        fs,
        ver,
        flags,
        max_readahead: max_readahead as usize,
        config,
    })
}

async fn read_init_req(dev: AsyncArc<DevFuse>) -> Result<InitReqRaw> {
    compio::runtime::spawn_blocking(move || read_init_req_sync(&dev))
        .await
        .unwrap()
}

fn read_init_req_sync(dev: &DevFuse) -> Result<InitReqRaw> {
    #[repr(C, align(8))]
    struct Buf([u8; MIN_READ_BUFFER]);

    let mut buf = Buf([0u8; MIN_READ_BUFFER]);

    let len = loop {
        match nix::unistd::read(dev.as_fd(), &mut buf.0) {
            Ok(len) => break len,
            // ENOENT means the request was interrupted and the read can be retried.
            Err(nix::Error::EINTR | nix::Error::EAGAIN | nix::Error::ENOENT) => {}
            Err(err) => {
                log::error!("failed to read FUSE device: {err}");
                return Err(err.into());
            }
        }
    };

    if len < size_of::<ReqHdr>() {
        log::error!("incomplete INIT request received from FUSE device: {len} bytes read");

        return Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "short read from FUSE device",
        ));
    }

    // SAFETY: `buf` is larger than `InitReqRaw`, whose fields are all integers,
    // and any bytes past `len` are zeroed.
    Ok(unsafe { std::ptr::read(buf.0.as_ptr().cast()) })
}

async fn write_init_err(dev: AsyncArc<DevFuse>, unique: u64, err: crate::Error) -> Result<()> {
    // Error replies must consist of only the header.
    write_init_resp(
        dev,
        InitRespRaw {
            hdr: RespHdr {
                len: const { size_of::<RespHdr>() as u32 },
                err: -err.wire_errno(),
                unique,
            },
            ..InitRespRaw::default()
        },
    )
    .await
}

async fn write_init_resp(dev: AsyncArc<DevFuse>, resp: InitRespRaw) -> Result<()> {
    compio::runtime::spawn_blocking(move || write_init_resp_sync(&dev, resp))
        .await
        .unwrap()
}

fn write_init_resp_sync(dev: &DevFuse, resp: InitRespRaw) -> Result<()> {
    let len = resp.hdr.len as usize;

    if len > size_of::<InitRespRaw>() {
        return Err(ErrorKind::InvalidInput.into());
    }

    // SAFETY: `InitRespRaw` is `repr(C)` with no padding (asserted above), so its
    // first `len` bytes are initialized.
    let buf =
        unsafe { std::slice::from_raw_parts((&resp as *const InitRespRaw).cast::<u8>(), len) };

    loop {
        match nix::unistd::write(dev.as_fd(), buf) {
            Ok(_) => return Ok(()),
            Err(nix::Error::EINTR | nix::Error::EAGAIN) => {}
            Err(err) => {
                log::error!("failed to write to FUSE device: {err}");
                return Err(err.into());
            }
        }
    }
}
