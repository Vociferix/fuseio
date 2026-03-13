use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MountOpt {
    Rw,
    Ro,
    Suid,
    NoSuid,
    Dev,
    NoDev,
    Exec,
    NoExec,
    Async,
    Sync,
    Atime,
    NoAtime,
    DirAtime,
    NoDirAtime,
    LazyTime,
    NoLazyTime,
    RelAtime,
    NoRelAtime,
    StrictAtime,
    NoStrictAtime,
    DirSync,
    SymFollow,
    NoSymFollow,
    AllowOther,
    AllowRoot,
    DefaultPermissions,
    BlockDev,
    LargeRead,
    FsName(OsString),
    SubType(OsString),
    MaxRead(usize),
    BlockSize(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ParseMountOptError;

impl std::fmt::Display for MountOpt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rw => f.write_str("rw"),
            Self::Ro => f.write_str("ro"),
            Self::Suid => f.write_str("suid"),
            Self::NoSuid => f.write_str("nosuid"),
            Self::Dev => f.write_str("dev"),
            Self::NoDev => f.write_str("nodev"),
            Self::Exec => f.write_str("exec"),
            Self::NoExec => f.write_str("noexec"),
            Self::Async => f.write_str("async"),
            Self::Sync => f.write_str("sync"),
            Self::Atime => f.write_str("atime"),
            Self::NoAtime => f.write_str("noatime"),
            Self::DirAtime => f.write_str("diratime"),
            Self::NoDirAtime => f.write_str("nodiratime"),
            Self::LazyTime => f.write_str("lazytime"),
            Self::NoLazyTime => f.write_str("nolazytime"),
            Self::RelAtime => f.write_str("relatime"),
            Self::NoRelAtime => f.write_str("norelatime"),
            Self::StrictAtime => f.write_str("strictatime"),
            Self::NoStrictAtime => f.write_str("nostrictatime"),
            Self::DirSync => f.write_str("dirsync"),
            Self::SymFollow => f.write_str("symfollow"),
            Self::NoSymFollow => f.write_str("nosymfollow"),
            Self::AllowOther => f.write_str("allow_other"),
            Self::AllowRoot => f.write_str("allow_root"),
            Self::DefaultPermissions => f.write_str("default_permissions"),
            Self::BlockDev => f.write_str("blkdev"),
            Self::LargeRead => f.write_str("large_read"),
            Self::FsName(name) => write!(f, "fsname={}", name.display()),
            Self::SubType(subtype) => write!(f, "subtype={}", subtype.display()),
            Self::MaxRead(max) => write!(f, "max_read={max}"),
            Self::BlockSize(blksize) => write!(f, "blksize={blksize}"),
        }
    }
}

impl std::str::FromStr for MountOpt {
    type Err = ParseMountOptError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((key, val)) = s.split_once('=') {
            const MAX_LEN: usize = b"max_read".len();

            if key.len() > MAX_LEN {
                return Err(ParseMountOptError);
            }

            let mut buf = [0u8; MAX_LEN];
            for (src, dst) in key.as_bytes().iter().copied().zip(buf.iter_mut()) {
                *dst = src.to_ascii_lowercase();
            }

            match &buf[..key.len()] {
                b"fsname" => Ok(Self::FsName(val.into())),
                b"subtype" => Ok(Self::SubType(val.into())),
                b"max_read" => val.parse().map_or_else(
                    |_| Err(ParseMountOptError),
                    |max_read| Ok(Self::MaxRead(max_read)),
                ),
                b"blksize" => val.parse().map_or_else(
                    |_| Err(ParseMountOptError),
                    |blksize| Ok(Self::BlockSize(blksize)),
                ),
                _ => Err(ParseMountOptError),
            }
        } else {
            const MAX_LEN: usize = b"default_permissions".len();

            if s.len() > MAX_LEN {
                return Err(ParseMountOptError);
            }

            let mut buf = [0u8; MAX_LEN];
            for (src, dst) in s.as_bytes().iter().copied().zip(buf.iter_mut()) {
                *dst = src.to_ascii_lowercase();
            }

            Ok(match &buf[..s.len()] {
                b"rw" => Self::Rw,
                b"ro" => Self::Ro,
                b"suid" => Self::Suid,
                b"nosuid" => Self::NoSuid,
                b"dev" => Self::Dev,
                b"nodev" => Self::NoDev,
                b"exec" => Self::Exec,
                b"noexec" => Self::NoExec,
                b"async" => Self::Async,
                b"sync" => Self::Sync,
                b"atime" => Self::Atime,
                b"noatime" => Self::NoAtime,
                b"diratime" => Self::DirAtime,
                b"nodiratime" => Self::NoDirAtime,
                b"lazytime" => Self::LazyTime,
                b"nolazytime" => Self::NoLazyTime,
                b"relatime" => Self::RelAtime,
                b"norelatime" => Self::NoRelAtime,
                b"strictatime" => Self::StrictAtime,
                b"nostrictatime" => Self::NoStrictAtime,
                b"dirsync" => Self::DirSync,
                b"symfollow" => Self::SymFollow,
                b"nosymfollow" => Self::NoSymFollow,
                b"allow_other" => Self::AllowOther,
                b"allow_root" => Self::AllowRoot,
                b"default_permissions" => Self::DefaultPermissions,
                b"blkdev" => Self::BlockDev,
                b"large_read" => Self::LargeRead,
                _ => return Err(ParseMountOptError),
            })
        }
    }
}

impl std::fmt::Display for ParseMountOptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid mount option")
    }
}

impl std::error::Error for ParseMountOptError {}
