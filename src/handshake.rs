use crate::MountOpt;
use crate::async_arc::AsyncArc;
use crate::dev_fuse::DevFuse;
use crate::fs::MountFs;
use crate::types::{FsCaps, KernelCaps, ReplyInitFlags, Version};

use std::io::Result;
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
    max_background: u16,
    congestion_threadhold: u16,
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

const MAJOR_VER: u32 = 7;
const MINOR_VER: u32 = 45;
const INIT_OPCODE: u32 = 26;

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
        todo!()
    }
}

impl Config {}

pub async fn handshake<F: MountFs>(
    fs: F,
    dev: AsyncArc<DevFuse>,
    opts: &[MountOpt],
) -> Result<Init<F::Fs>> {
    loop {
        let msg = read_init_req(dev.clone()).await?;
        if (msg.hdr.len as usize) < std::mem::size_of::<ReqHdr>()
            || msg.hdr.opcode != INIT_OPCODE
            || msg.major < MAJOR_VER
        {
            let _ = write_init_resp(
                dev.clone(),
                InitRespRaw {
                    hdr: RespHdr {
                        len: const { std::mem::size_of::<RespHdr>() as u32 },
                        err: -crate::Error::EPROTO.raw_os_error(),
                        unique: msg.hdr.unique,
                    },
                    ..InitRespRaw::default()
                },
            )
            .await;

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid initialization request from kernel",
            ));
        }

        if msg.major > MAJOR_VER {
            write_init_resp(
                dev.clone(),
                InitRespRaw {
                    hdr: RespHdr {
                        len: const { std::mem::size_of::<InitRespRaw>() as u32 },
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

        todo!()
    }
}

async fn read_init_req(dev: AsyncArc<DevFuse>) -> Result<InitReqRaw> {
    compio::runtime::spawn_blocking(move || read_init_req_sync(&dev))
        .await
        .unwrap()
}

fn read_init_req_sync(dev: &DevFuse) -> Result<InitReqRaw> {
    let mut buf = [0u8; std::mem::size_of::<InitReqRaw>()];

    loop {
        match nix::unistd::read(dev.as_fd(), &mut buf) {
            Ok(_) => break,
            Err(err) if err == nix::Error::EAGAIN => {}
            Err(err) => return Err(err.into()),
        }
    }

    Ok(unsafe { std::ptr::read(buf.as_ptr() as *const InitReqRaw) })
}

async fn write_init_resp(dev: AsyncArc<DevFuse>, resp: InitRespRaw) -> Result<()> {
    compio::runtime::spawn_blocking(move || write_init_resp_sync(&dev, resp))
        .await
        .unwrap()
}

fn write_init_resp_sync(dev: &DevFuse, resp: InitRespRaw) -> Result<()> {
    if resp.hdr.len as usize > std::mem::size_of::<InitRespRaw>() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }

    let len = resp.hdr.len as usize;

    let buf = unsafe { std::slice::from_raw_parts(&resp as *const InitRespRaw as *const u8, len) };

    loop {
        match nix::unistd::write(dev.as_fd(), buf) {
            Ok(_) => break,
            Err(err) if err == nix::Error::EAGAIN => {}
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}
