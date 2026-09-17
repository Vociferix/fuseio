use crate::MountOpt;
use crate::async_arc::AsyncArc;
use crate::dev_fuse::DevFuse;
use crate::fs::{BindFs, MountFs};
use crate::types::{FsCaps, KernelCaps, KernelInitFlags, ReplyInitFlags, Version};

use std::io::{ErrorKind, Result};
use std::mem::size_of;
use std::os::fd::AsFd;

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
    map_alignment: u16,
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
    opcode: u32,
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

const MAJOR_VER: u32 = 7;
const MINOR_VER: u32 = 45;
const INIT_OPCODE: u32 = 26;

/// `FUSE_MIN_READ_BUFFER`: the kernel rejects device reads into smaller buffers.
const MIN_READ_BUFFER: usize = 8192;

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
            max_write: crate::MAX_WRITE_SIZE as u32,
            time_gran: 1,
            map_alignment: 0,
        }
    }
}

impl Config {
    // TODO
}

pub async fn handshake<F: MountFs>(
    fs: F,
    dev: AsyncArc<DevFuse>,
    opts: &[MountOpt],
) -> Result<Init<F::Fs>> {
    let msg = loop {
        let msg = read_init_req(dev.clone()).await?;

        // `max_readahead` and `flags` only exist from 7.6.
        let min_len = size_of::<ReqHdr>() + if msg.minor >= 6 { 16 } else { 8 };

        if msg.hdr.opcode != INIT_OPCODE
            || (msg.hdr.len as usize) < min_len
            || msg.major < MAJOR_VER
        {
            let _ = write_init_err(dev.clone(), msg.hdr.unique, crate::Error::EPROTO).await;

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
        map_alignment: config.map_alignment,
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

    Ok(Init {
        fs,
        ver: Version(MAJOR_VER, msg.minor.min(MINOR_VER)),
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
            Err(err) => return Err(err.into()),
        }
    };

    if len < size_of::<ReqHdr>() {
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
                err: -err.raw_os_error(),
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
            Err(err) => return Err(err.into()),
        }
    }
}
