use super::{Cfg, HDR_LEN, Ino};
use crate::types::OFlag;
use crate::{Error, Result, buf::Buf};

/// `FUSE_OPEN_KILL_SUIDGID`.
pub(super) const KILL_SUIDGID: u32 = 1 << 0;

#[derive(Debug)]
pub struct Open {
    ino: Ino,
    flags: OFlag,
    remove_suid_sgid: bool,
}

#[repr(C)]
struct Raw {
    flags: u32,
    open_flags: u32,
}

impl Open {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    /// The flags the file was opened with.
    pub fn open_flags(&self) -> OFlag {
        self.flags
    }

    /// Whether to clear the setuid bit, and the setgid bit if the file is
    /// group-executable.
    pub fn remove_suid_sgid(&self) -> bool {
        self.remove_suid_sgid
    }
}

impl Open {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            remove_suid_sgid: raw.open_flags & KILL_SUIDGID != 0,
        })
    }
}
