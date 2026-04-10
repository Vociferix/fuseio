use super::{Body, Ino, Request, decode, handle_error, send_result};
use crate::async_rc::AsyncRc;
use crate::layout::{SetXattrIn, SetXattrInExt};
use crate::serve::Server;
use crate::{Filesystem, Result};

use compio::runtime::spawn;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct SetXattrReq<'a> {
    req: Request,
    ino: Ino,
    flags: SetXattrFlags,
    key: &'a OsStr,
    value: &'a [u8],
}

impl<'a> SetXattrReq<'a> {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn flags(&self) -> SetXattrFlags {
        self.flags
    }

    pub fn key(&self) -> &'a OsStr {
        self.key
    }

    pub fn value(&self) -> &'a [u8] {
        self.value
    }
}

impl std::ops::Deref for SetXattrReq<'_> {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct SetXattrFlags: u32 {
        const CLEAR_SGID = 1;
    }
}

impl Server {
    pub fn setxattr<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let (hdr, kv) = {
            #[cfg(target_os = "macos")]
            {
                let (hdr, kv) = decode::<SetXattrIn>(body)?;
                if hdr.position != 0 {
                    return Err(Error::EINVAL);
                }
                (SetXattrInExt::from_basic(hdr), kv)
            }

            #[cfg(not(target_os = "macos"))]
            {
                if self.want.contains(crate::InitFlags::SETXATTR_EXT) {
                    decode::<SetXattrInExt>(body)?
                } else {
                    let (hdr, kv) = decode::<SetXattrIn>(body)?;
                    (SetXattrInExt::from_basic(hdr), kv)
                }
            }
        };

        let key_len = memchr::memchr(0, &kv).unwrap_or(kv.len());
        let val_pos = if key_len == kv.len() {
            key_len
        } else {
            key_len + 1
        };
        let val_len = memchr::memchr(0, &kv[val_pos..]).unwrap_or(kv.len() - val_pos);

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = SetXattrReq {
                req,
                ino,
                flags: SetXattrFlags::from_bits_retain(hdr.flags),
                key: OsStr::from_bytes(&kv[..key_len]),
                value: &kv[val_pos..][..val_len],
            };

            handle_error(send_result(fs.set_xattr(ureq).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
