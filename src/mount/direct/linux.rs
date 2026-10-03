//! The Linux half of [`DirectMount`](super::DirectMount).
//!
//! `mount(2)` is told the device descriptor, so the kernel takes the open file
//! rather than opening anything itself. Everything else is either an `MS_*`
//! flag or one of the ten options `fuse_fs_parameters` accepts
//! (`fs/fuse/inode.c`).
//!
//! What libfuse's `fusermount` does that is deliberately not repeated here:
//!
//! * The mountpoint ownership, permission and symlink checks. `fusermount` runs
//!   as root and must decide for itself whether the real user may mount over a
//!   path; `mount(2)` applies this caller's own credentials.
//! * `/etc/fuse.conf` and its `user_allow_other`. That file is `fusermount`'s
//!   policy for granting mount privileges to users who lack them, and it has no
//!   bearing on a caller that already has them.
//! * The `blkdev` privilege check. The kernel refuses `fuseblk` without
//!   `CAP_SYS_ADMIN` on its own.
//! * Updating `/etc/mtab` or `/run/mount/utab`. libfuse does this by spawning
//!   `/bin/mount`, which is the kind of subprocess this mounter exists to
//!   avoid. The kernel's own `/proc/self/mounts` still lists the mount, so
//!   `mount`, `df` and `findmnt` all see it; what is missing is the `user=`
//!   annotation that lets an unprivileged `umount` succeed later.
//! * The retry that drops `group_id=` when `mount(2)` returns `EINVAL`. That
//!   works around kernels predating the option, which is to say before 2.6, and
//!   on anything newer it would turn a real complaint about the options into a
//!   second confusing failure.

use crate::MountOpt;

use nix::errno::Errno;
use nix::mount::{MntFlags, MsFlags, mount, umount2};

use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::io::Result;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::path::Path;

/// `MS_NOSYMFOLLOW` from `include/uapi/linux/mount.h` (Linux 5.10), which
/// neither `nix` nor `libc` defines. An older kernel ignores the bit.
const MS_NOSYMFOLLOW: MsFlags = MsFlags::from_bits_retain(1 << 8);

/// What the options asked for, split the way `mount(2)` wants it.
struct Params {
    /// The `MS_*` bitmask.
    flags: MsFlags,
    /// Goes in the `source` argument, so a comma or a backslash in it needs no
    /// escaping, unlike the option list `fusermount` has to build.
    fsname: Option<OsString>,
    /// Goes in the filesystem type, as `fuse.<subtype>`.
    subtype: Option<OsString>,
    blkdev: bool,
    allow_other: bool,
    default_permissions: bool,
    max_read: Option<usize>,
    blksize: Option<usize>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            // libfuse's default for both of its mount paths: a filesystem
            // served by a userspace process is not somewhere to go looking for
            // setuid binaries or device nodes unless the caller insists.
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
            fsname: None,
            subtype: None,
            blkdev: false,
            allow_other: false,
            default_permissions: false,
            max_read: None,
            blksize: None,
        }
    }
}

/// Mounts the filesystem the device belongs to at `mountpoint`.
pub(super) fn attach(
    dev: BorrowedFd<'_>,
    device: &Path,
    mountpoint: &Path,
    options: &[MountOpt],
) -> Result<()> {
    let params = parse(options);
    let data = data(&params, dev, root_mode(mountpoint)?);

    let res = mount(
        Some(&*source(&params, device, false)),
        mountpoint,
        Some(&*fstype(&params, true)),
        params.flags,
        Some(data.as_str()),
    );

    match res {
        Ok(()) => return Ok(()),
        // A kernel built without subtype support does not recognise the
        // `fuse.<subtype>` type. libfuse retries with the bare type and the
        // subtype folded into the source instead.
        Err(Errno::ENODEV) if params.subtype.is_some() => {}
        Err(err) => return Err(err.into()),
    }

    mount(
        Some(&*source(&params, device, true)),
        mountpoint,
        Some(&*fstype(&params, false)),
        params.flags,
        Some(data.as_str()),
    )
    .map_err(Into::into)
}

/// Unmounts the filesystem at `mountpoint`.
///
/// The caller has already closed the device, so the filesystem can no longer
/// answer anything by the time this runs.
pub(super) fn detach(mountpoint: &Path) -> Result<()> {
    // `UMOUNT_NOFOLLOW`, because the mountpoint is named by path and a symlink
    // swapped in underneath should not redirect the unmount somewhere else.
    match umount2(mountpoint, MntFlags::UMOUNT_NOFOLLOW) {
        Ok(()) => Ok(()),
        // Something is still using the mountpoint -- a working directory inside
        // it is enough. Detaching takes it out of the namespace now and lets
        // the kernel clean up when the last user leaves, which leaves less
        // behind than a mount whose server has gone.
        Err(Errno::EBUSY) => umount2(mountpoint, MntFlags::MNT_DETACH | MntFlags::UMOUNT_NOFOLLOW)
            .map_err(Into::into),
        Err(err) => Err(err.into()),
    }
}

/// Sorts the options into the three places `mount(2)` takes them.
fn parse(options: &[MountOpt]) -> Params {
    let mut params = Params::default();

    // `set` rather than `insert`/`remove` so each option reads as the flag it
    // names and the direction it sets.
    let mut set = |flag, on| params.flags.set(flag, on);

    for option in options {
        match option {
            MountOpt::Ro => set(MsFlags::MS_RDONLY, true),
            MountOpt::Rw => set(MsFlags::MS_RDONLY, false),
            MountOpt::Suid => set(MsFlags::MS_NOSUID, false),
            MountOpt::NoSuid => set(MsFlags::MS_NOSUID, true),
            MountOpt::Dev => set(MsFlags::MS_NODEV, false),
            MountOpt::NoDev => set(MsFlags::MS_NODEV, true),
            MountOpt::Exec => set(MsFlags::MS_NOEXEC, false),
            MountOpt::NoExec => set(MsFlags::MS_NOEXEC, true),
            MountOpt::Async => set(MsFlags::MS_SYNCHRONOUS, false),
            MountOpt::Sync => set(MsFlags::MS_SYNCHRONOUS, true),
            MountOpt::Atime => set(MsFlags::MS_NOATIME, false),
            MountOpt::NoAtime => set(MsFlags::MS_NOATIME, true),
            MountOpt::DirAtime => set(MsFlags::MS_NODIRATIME, false),
            MountOpt::NoDirAtime => set(MsFlags::MS_NODIRATIME, true),
            MountOpt::LazyTime => set(MsFlags::MS_LAZYTIME, true),
            MountOpt::NoLazyTime => set(MsFlags::MS_LAZYTIME, false),
            MountOpt::RelAtime => set(MsFlags::MS_RELATIME, true),
            MountOpt::NoRelAtime => set(MsFlags::MS_RELATIME, false),
            MountOpt::StrictAtime => set(MsFlags::MS_STRICTATIME, true),
            MountOpt::NoStrictAtime => set(MsFlags::MS_STRICTATIME, false),
            MountOpt::DirSync => set(MsFlags::MS_DIRSYNC, true),
            MountOpt::SymFollow => set(MS_NOSYMFOLLOW, false),
            MountOpt::NoSymFollow => set(MS_NOSYMFOLLOW, true),

            MountOpt::AllowOther => params.allow_other = true,
            // `allow_root` is not a mount option on any platform. libfuse mounts
            // with `allow_other` and then rejects requests whose uid is neither
            // the owner's nor root's, which is a decision for the layer reading
            // requests rather than for this one.
            // TODO: Nothing performs that check yet, so this currently grants
            //       the same access as `AllowOther`. See `TODO.md`.
            MountOpt::AllowRoot => params.allow_other = true,
            MountOpt::DefaultPermissions => params.default_permissions = true,
            MountOpt::BlockDev => params.blkdev = true,
            MountOpt::MaxRead(max) => params.max_read = Some(*max),
            MountOpt::BlockSize(size) => params.blksize = Some(*size),
            MountOpt::FsName(name) => params.fsname = Some(name.clone()),
            MountOpt::SubType(subtype) => params.subtype = Some(subtype.clone()),
        }
    }

    params
}

/// Builds the `data` argument: the options the kernel parses itself.
///
/// Every part of this is ASCII and bounded -- four numbers, an octal mode and
/// at most two fixed words -- so the silent truncation at `PAGE_SIZE` that
/// `fusermount` has to guard against cannot be reached. `fsname` and `subtype`,
/// the two options a caller controls the length of, are carried elsewhere.
fn data(params: &Params, dev: BorrowedFd<'_>, rootmode: nix::libc::mode_t) -> String {
    let mut out = String::new();

    if params.default_permissions {
        out.push_str("default_permissions,");
    }
    if params.allow_other {
        out.push_str("allow_other,");
    }
    if let Some(max) = params.max_read {
        let _ = write!(out, "max_read={max},");
    }
    // The kernel accepts this only for `fuseblk` and says so; passing it on
    // regardless keeps that complaint pointed at the option the caller gave.
    if let Some(size) = params.blksize {
        let _ = write!(out, "blksize={size},");
    }

    let _ = write!(
        out,
        "fd={},rootmode={:o},user_id={},group_id={}",
        dev.as_raw_fd(),
        rootmode,
        nix::unistd::getuid().as_raw(),
        nix::unistd::getgid().as_raw(),
    );

    out
}

/// The file type bits of the mountpoint, which the kernel wants for the root
/// inode it is about to create.
///
/// `mount(2)` resolves the path again afterwards, so a path that changes in
/// between would be described by a stale mode. Pinning it with `O_PATH` the way
/// `fusermount` does would close that window, but it only matters to a caller
/// racing against itself over its own mountpoint.
fn root_mode(mountpoint: &Path) -> Result<nix::libc::mode_t> {
    let stat = nix::sys::stat::stat(mountpoint)?;

    Ok(stat.st_mode & nix::libc::S_IFMT)
}

/// The `source` argument, which is what `/proc/self/mounts` reports.
///
/// `prefix_subtype` is for the retry above: with the subtype gone from the
/// type, `subtype#fsname` is where libfuse puts it instead.
fn source(params: &Params, device: &Path, prefix_subtype: bool) -> OsString {
    match (&params.fsname, &params.subtype) {
        (Some(fsname), Some(subtype)) if prefix_subtype && !params.blkdev => {
            let mut out = subtype.clone();
            out.push("#");
            out.push(fsname);
            out
        }
        (Some(fsname), _) => fsname.clone(),
        (None, Some(subtype)) => subtype.clone(),
        (None, None) => device.as_os_str().to_os_string(),
    }
}

/// The filesystem type, which is `fuse` or `fuseblk`, optionally with the
/// subtype after a dot.
fn fstype(params: &Params, with_subtype: bool) -> OsString {
    let mut out = OsString::from(if params.blkdev { "fuseblk" } else { "fuse" });

    if with_subtype && let Some(subtype) = &params.subtype {
        out.push(".");
        out.push(subtype);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::fd::AsFd;

    /// A real descriptor, since the number only means anything while one is
    /// open. Which number it gets is up to the process.
    fn somefd() -> std::fs::File {
        std::fs::File::open("/dev/null").expect("/dev/null")
    }

    // A FUSE mount is served by an ordinary process, so a setuid binary or a
    // device node found inside one is not to be trusted. Both of libfuse's
    // mount paths start here too.
    #[test]
    fn nothing_asked_for_still_means_nosuid_and_nodev() {
        let params = parse(&[]);

        assert_eq!(params.flags, MsFlags::MS_NOSUID | MsFlags::MS_NODEV);
    }

    #[test]
    fn asking_for_suid_and_dev_takes_the_defaults_back_off() {
        let params = parse(&[MountOpt::Suid, MountOpt::Dev]);

        assert!(params.flags.is_empty(), "{:?}", params.flags);
    }

    // The option names a direction, and the later one wins.
    #[test]
    fn the_last_option_decides() {
        let params = parse(&[MountOpt::Ro, MountOpt::Rw]);

        assert!(!params.flags.contains(MsFlags::MS_RDONLY));

        let params = parse(&[MountOpt::Rw, MountOpt::Ro]);

        assert!(params.flags.contains(MsFlags::MS_RDONLY));
    }

    // `allow_root` is not a mount option anywhere; the kernel half of it is
    // `allow_other`, and the uid check belongs to whoever reads requests.
    #[test]
    fn allow_root_asks_the_kernel_for_allow_other() {
        let params = parse(&[MountOpt::AllowRoot]);

        assert!(params.allow_other);
    }

    // The kernel takes it as a flag, so saying it twice would be harmless, but
    // an option list that repeats itself is still worth not sending.
    #[test]
    fn allow_other_and_allow_root_together_say_it_once() {
        let params = parse(&[MountOpt::AllowOther, MountOpt::AllowRoot]);
        let fd = somefd();
        let rendered = data(&params, fd.as_fd(), 0o40000);

        assert_eq!(rendered.matches("allow_other").count(), 1, "{rendered}");
    }

    // The kernel parses this left to right, and the four mandatory options are
    // what the mount cannot do without, so they go last where nothing can get
    // between them.
    #[test]
    fn the_data_ends_with_what_the_mount_cannot_do_without() {
        let params = parse(&[MountOpt::DefaultPermissions, MountOpt::MaxRead(4096)]);
        let fd = somefd();
        let rendered = data(&params, fd.as_fd(), 0o40000);

        let tail = format!(
            "fd={},rootmode=40000,user_id={},group_id={}",
            fd.as_fd().as_raw_fd(),
            nix::unistd::getuid().as_raw(),
            nix::unistd::getgid().as_raw(),
        );

        assert_eq!(
            rendered,
            format!("default_permissions,max_read=4096,{tail}"),
            "{rendered}"
        );
    }

    #[test]
    fn the_source_is_the_device_when_nothing_names_it() {
        let params = parse(&[]);

        assert_eq!(
            source(&params, Path::new("/dev/fuse"), false),
            OsStr::new("/dev/fuse")
        );
    }

    #[test]
    fn a_subtype_alone_names_the_source() {
        let params = parse(&[MountOpt::SubType("demo".into())]);

        assert_eq!(
            source(&params, Path::new("/dev/fuse"), false),
            OsStr::new("demo")
        );
    }

    // The retry for a kernel without subtype support: the type loses the
    // subtype, so the source carries it instead.
    #[test]
    fn the_retry_folds_the_subtype_into_the_source() {
        let params = parse(&[
            MountOpt::FsName("myfs".into()),
            MountOpt::SubType("demo".into()),
        ]);

        assert_eq!(
            source(&params, Path::new("/dev/fuse"), false),
            OsStr::new("myfs")
        );
        assert_eq!(
            source(&params, Path::new("/dev/fuse"), true),
            OsStr::new("demo#myfs")
        );
    }

    // `fuseblk` names a real block device, so there is no room to put the
    // subtype in front of it.
    #[test]
    fn a_block_device_source_is_left_alone_by_the_retry() {
        let params = parse(&[
            MountOpt::BlockDev,
            MountOpt::FsName("/dev/sda1".into()),
            MountOpt::SubType("demo".into()),
        ]);

        assert_eq!(
            source(&params, Path::new("/dev/fuse"), true),
            OsStr::new("/dev/sda1")
        );
    }

    // The source goes to `mount(2)` as its own argument rather than into the
    // comma-separated options, so nothing in it needs escaping and nothing in
    // it has to be UTF-8.
    #[test]
    fn a_source_that_isnt_utf8_survives_intact() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let name = OsString::from_vec(vec![b'a', b',', 0xff, b'\\', b'b']);
        let params = parse(&[MountOpt::FsName(name)]);

        assert_eq!(
            source(&params, Path::new("/dev/fuse"), false).as_bytes(),
            b"a,\xff\\b"
        );
    }

    #[test]
    fn the_type_says_fuse_or_fuseblk_and_then_the_subtype() {
        let plain = parse(&[]);
        let sub = parse(&[MountOpt::SubType("demo".into())]);
        let blk = parse(&[MountOpt::BlockDev]);
        let both = parse(&[MountOpt::BlockDev, MountOpt::SubType("demo".into())]);

        assert_eq!(fstype(&plain, true), OsStr::new("fuse"));
        assert_eq!(fstype(&sub, true), OsStr::new("fuse.demo"));
        assert_eq!(fstype(&blk, true), OsStr::new("fuseblk"));
        assert_eq!(fstype(&both, true), OsStr::new("fuseblk.demo"));

        // The retry drops it.
        assert_eq!(fstype(&sub, false), OsStr::new("fuse"));
    }

    #[test]
    fn the_root_mode_is_the_file_type_of_the_mountpoint() {
        let dir = std::env::temp_dir();

        assert_eq!(root_mode(&dir).unwrap(), nix::libc::S_IFDIR);
    }

    #[test]
    fn a_mountpoint_that_isnt_there_is_reported() {
        let err = root_mode(Path::new("/nonexistent/fuseio-direct")).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
