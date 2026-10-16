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

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    /// Every option, so a new variant without a test here fails to compile.
    fn every_option() -> Vec<MountOpt> {
        let all = [
            MountOpt::Rw,
            MountOpt::Ro,
            MountOpt::Suid,
            MountOpt::NoSuid,
            MountOpt::Dev,
            MountOpt::NoDev,
            MountOpt::Exec,
            MountOpt::NoExec,
            MountOpt::Async,
            MountOpt::Sync,
            MountOpt::Atime,
            MountOpt::NoAtime,
            MountOpt::DirAtime,
            MountOpt::NoDirAtime,
            MountOpt::LazyTime,
            MountOpt::NoLazyTime,
            MountOpt::RelAtime,
            MountOpt::NoRelAtime,
            MountOpt::StrictAtime,
            MountOpt::NoStrictAtime,
            MountOpt::DirSync,
            MountOpt::SymFollow,
            MountOpt::NoSymFollow,
            MountOpt::AllowOther,
            MountOpt::AllowRoot,
            MountOpt::DefaultPermissions,
            MountOpt::BlockDev,
        ];

        // Exhaustive by construction: this fails to build if a variant is added
        // without being listed above.
        for opt in &all {
            match opt {
                MountOpt::Rw
                | MountOpt::Ro
                | MountOpt::Suid
                | MountOpt::NoSuid
                | MountOpt::Dev
                | MountOpt::NoDev
                | MountOpt::Exec
                | MountOpt::NoExec
                | MountOpt::Async
                | MountOpt::Sync
                | MountOpt::Atime
                | MountOpt::NoAtime
                | MountOpt::DirAtime
                | MountOpt::NoDirAtime
                | MountOpt::LazyTime
                | MountOpt::NoLazyTime
                | MountOpt::RelAtime
                | MountOpt::NoRelAtime
                | MountOpt::StrictAtime
                | MountOpt::NoStrictAtime
                | MountOpt::DirSync
                | MountOpt::SymFollow
                | MountOpt::NoSymFollow
                | MountOpt::AllowOther
                | MountOpt::AllowRoot
                | MountOpt::DefaultPermissions
                | MountOpt::BlockDev => {}
                MountOpt::FsName(_)
                | MountOpt::SubType(_)
                | MountOpt::MaxRead(_)
                | MountOpt::BlockSize(_) => unreachable!("value options are tested separately"),
            }
        }

        all.to_vec()
    }

    // These names go to `fusermount` and into `mount(2)`'s option string, so
    // they are an interface rather than a label: a typo would ask for the wrong
    // thing, or nothing.
    #[test]
    fn the_names_are_the_ones_mount_understands() {
        let expected = [
            (MountOpt::Rw, "rw"),
            (MountOpt::Ro, "ro"),
            (MountOpt::Suid, "suid"),
            (MountOpt::NoSuid, "nosuid"),
            (MountOpt::Dev, "dev"),
            (MountOpt::NoDev, "nodev"),
            (MountOpt::Exec, "exec"),
            (MountOpt::NoExec, "noexec"),
            (MountOpt::Async, "async"),
            (MountOpt::Sync, "sync"),
            (MountOpt::Atime, "atime"),
            (MountOpt::NoAtime, "noatime"),
            (MountOpt::DirAtime, "diratime"),
            (MountOpt::NoDirAtime, "nodiratime"),
            (MountOpt::LazyTime, "lazytime"),
            (MountOpt::NoLazyTime, "nolazytime"),
            (MountOpt::RelAtime, "relatime"),
            (MountOpt::NoRelAtime, "norelatime"),
            (MountOpt::StrictAtime, "strictatime"),
            (MountOpt::NoStrictAtime, "nostrictatime"),
            (MountOpt::DirSync, "dirsync"),
            (MountOpt::SymFollow, "symfollow"),
            (MountOpt::NoSymFollow, "nosymfollow"),
            (MountOpt::AllowOther, "allow_other"),
            (MountOpt::AllowRoot, "allow_root"),
            (MountOpt::DefaultPermissions, "default_permissions"),
            (MountOpt::BlockDev, "blkdev"),
        ];

        for (opt, name) in expected {
            assert_eq!(opt.to_string(), name, "{opt:?}");
        }
    }

    #[test]
    fn every_flag_survives_a_round_trip() {
        for opt in every_option() {
            let rendered = opt.to_string();

            assert_eq!(
                MountOpt::from_str(&rendered).ok(),
                Some(opt.clone()),
                "{rendered} did not parse back"
            );
        }
    }

    #[test]
    fn the_options_carrying_a_value_survive_a_round_trip() {
        let values = [
            MountOpt::FsName("myfs".into()),
            MountOpt::SubType("demo".into()),
            MountOpt::MaxRead(65536),
            MountOpt::BlockSize(512),
        ];

        for opt in values {
            let rendered = opt.to_string();

            assert_eq!(
                MountOpt::from_str(&rendered).ok(),
                Some(opt.clone()),
                "{rendered}"
            );
        }
    }

    #[test]
    fn the_value_options_render_as_key_equals_value() {
        assert_eq!(MountOpt::FsName("myfs".into()).to_string(), "fsname=myfs");
        assert_eq!(MountOpt::SubType("demo".into()).to_string(), "subtype=demo");
        assert_eq!(MountOpt::MaxRead(4096).to_string(), "max_read=4096");
        assert_eq!(MountOpt::BlockSize(512).to_string(), "blksize=512");
    }

    // No two options may render alike, or parsing one would hand back the
    // other.
    #[test]
    fn no_two_options_share_a_name() {
        let mut names: Vec<String> = every_option().iter().map(ToString::to_string).collect();
        let before = names.len();

        names.sort();
        names.dedup();

        assert_eq!(names.len(), before, "two options render the same way");
    }

    #[test]
    fn something_that_is_not_an_option_is_refused() {
        for text in ["", "nonsense", "rw,ro", "max_read", "max_read=x", "fsname"] {
            assert!(MountOpt::from_str(text).is_err(), "{text:?} parsed");
        }
    }
}
