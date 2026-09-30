//! Mounting by way of libfuse's `fusermount` helper.
//!
//! The helper is setuid root, so this is how an unprivileged process mounts. It
//! opens `/dev/fuse` and calls `mount(2)` itself, then hands the device back
//! over a Unix socket named to it through the environment. The protocol is
//! libfuse's, and `fusermount` and `fusermount3` both speak it.

use super::{Mount, Unmount};
use crate::MountOpt;
use crate::conn::{Conn, DevFuseConn, DevFuseSharedConn};

use nix::sys::socket::{
    AddressFamily, ControlMessageOwned, MsgFlags, SockFlag, SockType, recvmsg, socketpair,
};

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::io::{Error, ErrorKind, IoSliceMut, Result};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Where `fusermount` looks for the socket to send the device on.
const COMMFD_ENV: &str = "_FUSE_COMMFD";

/// The other end of that socket, which `fusermount` is told only so that a
/// file-descriptor leak checker can recognise it.
const COMMFD2_ENV: &str = "_FUSE_COMMFD2";

/// Tried in order when no program was named, newest first since this crate
/// speaks protocol 7.x. Both understand the same `-o`/`-u`/`-q`/`-z` and the
/// same socket protocol.
const PROGRAMS: [&str; 2] = ["fusermount3", "fusermount"];

/// Where distributions install the helper, searched before `PATH`.
const SYSTEM_DIRS: [&str; 4] = ["/bin", "/usr/bin", "/usr/local/bin", "/sbin"];

/// A [`Mount`] implementation that calls libfuse's `fusermount` executable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fusermount {
    cmd: Option<Cow<'static, Path>>,
}

/// The unmount half of [`Fusermount`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FusermountUnmount {
    cmd: Cow<'static, Path>,
}

impl Fusermount {
    pub const fn from_static(path: &'static str) -> Self {
        // SAFETY: On UNIX-like platforms, `Path` is just an arbitrary
        //         byte slice, and `str` is represented as a byte slice
        //         on all platforms. This crate only supports UNIX-like
        //         platforms, so this is always safe. If we support
        //         Windows somehow in the future, this will need to
        //         change.
        let path: &'static Path = unsafe { std::mem::transmute(path) };

        Self::from_static_path(path)
    }

    pub const fn from_static_path(path: &'static Path) -> Self {
        Self {
            cmd: Some(Cow::Borrowed(path)),
        }
    }

    /// Constructs a [`Fusermount`] mounter that will use the provided `fusermount` executable.
    ///
    /// If the provided path does not exist, or is not an excutable compatible with `fusermount`,
    /// an error will be returned when [`Mount::mount`] is called.
    pub fn from_path<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            cmd: Some(path.as_ref().to_path_buf().into()),
        }
    }

    /// Constructs a [`Fusermount`] mounter that will autodetect the `fusermount` executable.
    ///
    /// When mounting, this mounter will search common default system locations for the
    /// executable. If the executable is not found, it will then search other directories
    /// according to the `PATH` environment variable. If the executable is still not found,
    /// mounting results in an error. If this is not suitable, use [`Self::from_path`] to
    /// specify the executable location.
    pub const fn new() -> Self {
        Self { cmd: None }
    }

    /// The executable to run, searched for when none was named.
    fn program(&self) -> Result<Cow<'static, Path>> {
        if let Some(cmd) = &self.cmd {
            return Ok(cmd.clone());
        }

        let path = std::env::var_os("PATH").unwrap_or_default();

        for program in PROGRAMS {
            let system = SYSTEM_DIRS.iter().map(|dir| Path::new(dir).to_path_buf());
            let from_path = std::env::split_paths(&path);

            for dir in system.chain(from_path) {
                let candidate = dir.join(program);

                if is_executable(&candidate) {
                    return Ok(Cow::Owned(candidate));
                }
            }
        }

        Err(Error::new(
            ErrorKind::NotFound,
            "no fusermount executable found in the system directories or on PATH",
        ))
    }
}

impl<P: AsRef<Path>> From<P> for Fusermount {
    fn from(path: P) -> Self {
        Self::from_path(path)
    }
}

impl Mount for Fusermount {
    type SharedConn = DevFuseSharedConn;
    type Conn = DevFuseConn;
    type Unmount = FusermountUnmount;

    async fn mount(
        &self,
        mountpoint: &Path,
        options: &[MountOpt],
        num_workers: usize,
    ) -> Result<(
        Self::SharedConn,
        impl Future<Output = Result<Self::Unmount>>,
    )> {
        // The helper hands back one device, and further workers clone it
        // themselves, so it has no use for the count.
        let _ = num_workers;

        let cmd = self.program()?;
        let opts = render_options(options);

        let fd = spawn_and_receive(&cmd, &opts, mountpoint).await?;

        // Mounting finished before the device arrived, so there is nothing left
        // to wait for.
        Ok((
            DevFuseSharedConn::from_fd(fd)?,
            std::future::ready(Ok(FusermountUnmount { cmd })),
        ))
    }
}

impl Unmount for FusermountUnmount {
    type SharedConn = DevFuseSharedConn;
    type Conn = DevFuseConn;

    async fn unmount(
        self,
        conn: Conn<Self::SharedConn>,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()> {
        // `fusermount -u` takes only the mountpoint; the options described the
        // mount that is going away.
        let _ = options;

        // `POLLERR` on the device means the filesystem is already gone, either
        // unmounted from elsewhere or aborted through
        // `/sys/fs/fuse/connections`.
        let severed = is_severed(&conn);

        // Closing the device first is not optional: a synchronous unmount of a
        // filesystem whose server still holds the device recurses into that
        // filesystem and deadlocks.
        drop(conn);

        if severed {
            return Ok(());
        }

        // Unmounting directly avoids a fork, and works for root and for the
        // user who owns the mount. Detaching is lazy, so the call returns
        // without waiting on the filesystem. It does leave the helper's `utab`
        // entry behind, which is what the fallback below cleans up.
        #[cfg(any(target_os = "linux", target_os = "android"))]
        if detach(mountpoint).is_ok() {
            return Ok(());
        }

        run_helper(
            &self.cmd,
            &[OsStr::new("-u"), OsStr::new("-q"), OsStr::new("-z")],
            mountpoint,
        )
        .await
    }
}

/// Runs the helper and takes the device it sends back.
async fn spawn_and_receive(program: &Path, opts: &OsStr, mountpoint: &Path) -> Result<OwnedFd> {
    // A stream pair, because the helper sends the device as ancillary data
    // alongside one byte of payload.
    let (ours, theirs) = socketpair(
        AddressFamily::Unix,
        SockType::Stream,
        None,
        SockFlag::empty(),
    )?;

    // Only the helper's end may be inherited. Marking ours close-on-exec is
    // what libfuse achieves with a `posix_spawn` close action, and it needs no
    // code between fork and exec.
    nix::fcntl::fcntl(
        &ours,
        nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
    )?;

    let child = compio::process::Command::new(program)
        .arg("-o")
        .arg(opts)
        .arg("--")
        .arg(mountpoint)
        // Per-child rather than `setenv`, which libfuse has to use and which is
        // neither thread-safe nor undone afterwards.
        .env(COMMFD_ENV, theirs.as_raw_fd().to_string())
        .env(COMMFD2_ENV, ours.as_raw_fd().to_string())
        .spawn()?;

    // The helper has its own copy now, and it must be the only one: the receive
    // below reads end-of-file when every writer is gone, which is how a failed
    // mount is reported.
    drop(theirs);

    let received = receive_fd(ours).await;

    // Reaped either way, and its exit status is the better diagnostic when the
    // device never arrived.
    let status = child.wait().await?;

    match received {
        Ok(fd) => Ok(fd),
        Err(err) if status.success() => Err(err),
        Err(_) => Err(Error::other(format!(
            "{} failed to mount: {status}",
            program.display()
        ))),
    }
}

/// Reads one `SCM_RIGHTS` message and takes the descriptor out of it.
///
/// Takes the socket by value so that it is closed once the device is in hand.
async fn receive_fd(sock: OwnedFd) -> Result<OwnedFd> {
    // One-shot readiness rather than an attached source, so nothing outlives
    // this call on the runtime this happens to be running on.
    let sock = compio::runtime::fd::PollFd::new(sock)?;

    let mut byte = [0u8];
    let mut iov = [IoSliceMut::new(&mut byte)];
    let mut cmsg = nix::cmsg_space!(std::os::fd::RawFd);

    let msg = loop {
        // `MSG_CMSG_CLOEXEC` closes the window libfuse leaves between receiving
        // the device and marking it close-on-exec afterwards. `MSG_DONTWAIT`
        // keeps the call from parking this thread; waiting is the runtime's job
        // below.
        match recvmsg::<()>(
            sock.as_raw_fd(),
            &mut iov,
            Some(&mut cmsg),
            MsgFlags::MSG_CMSG_CLOEXEC | MsgFlags::MSG_DONTWAIT,
        ) {
            Ok(msg) => break msg,
            Err(nix::Error::EINTR) => continue,
            Err(nix::Error::EAGAIN) => {}
            Err(err) => return Err(err.into()),
        }

        sock.read_ready().await?;
    };

    if msg.bytes == 0 {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "fusermount closed the socket without sending the FUSE device",
        ));
    }

    for message in msg.cmsgs()? {
        if let ControlMessageOwned::ScmRights(fds) = message {
            let mut fds = fds.into_iter();

            let Some(fd) = fds.next() else {
                continue;
            };

            // SAFETY: The kernel just installed these descriptors in this
            //         process, and nothing else holds them.
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };

            // One is all the protocol sends; close anything further rather than
            // leaking it.
            for extra in fds {
                drop(unsafe { OwnedFd::from_raw_fd(extra) });
            }

            return Ok(fd);
        }
    }

    Err(Error::new(
        ErrorKind::InvalidData,
        "fusermount sent no FUSE device with its reply",
    ))
}

/// Runs the helper with the given arguments and waits for it.
async fn run_helper(program: &Path, args: &[&OsStr], mountpoint: &Path) -> Result<()> {
    let status = compio::process::Command::new(program)
        .args(args)
        .arg("--")
        .arg(mountpoint)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::other(format!(
            "{} failed to unmount {}: {status}",
            program.display(),
            mountpoint.display()
        )))
    }
}

/// Whether the connection has already been torn down elsewhere.
fn is_severed(conn: &Conn<DevFuseSharedConn>) -> bool {
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

    let mut fds = [PollFd::new(conn.as_fd(), PollFlags::empty())];

    match poll(&mut fds, PollTimeout::ZERO) {
        Ok(1) => fds[0]
            .revents()
            .is_some_and(|revents| revents.contains(PollFlags::POLLERR)),
        _ => false,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn detach(mountpoint: &Path) -> Result<()> {
    use std::ffi::CString;

    let path = CString::new(mountpoint.as_os_str().as_bytes())
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "mountpoint contained a NUL byte"))?;

    // SAFETY: `path` is a valid NUL-terminated string for the duration of the
    //         call, and the flags are constants.
    let res = unsafe {
        nix::libc::umount2(
            path.as_ptr(),
            nix::libc::MNT_DETACH | nix::libc::UMOUNT_NOFOLLOW,
        )
    };

    nix::errno::Errno::result(res)
        .map(|_| ())
        .map_err(Into::into)
}

/// Whether the path names something this process may execute.
fn is_executable(path: &Path) -> bool {
    nix::unistd::access(path, nix::unistd::AccessFlags::X_OK).is_ok()
}

/// Renders the options as the helper's `-o` argument.
///
/// Built as an [`OsString`] rather than through [`MountOpt`]'s [`Display`], whose
/// lossy rendering would corrupt a name that isn't UTF-8.
///
/// [`Display`]: std::fmt::Display
fn render_options(options: &[MountOpt]) -> OsString {
    let mut out = OsString::new();

    for option in options {
        if !out.is_empty() {
            out.push(",");
        }

        match option {
            MountOpt::FsName(name) => {
                out.push("fsname=");
                out.push(escape(name));
            }
            MountOpt::SubType(subtype) => {
                out.push("subtype=");
                out.push(escape(subtype));
            }
            // Every other option renders as ASCII.
            option => out.push(option.to_string()),
        }
    }

    out
}

/// Escapes a value for a comma-separated option list, as the helper unescapes.
fn escape(value: &OsStr) -> OsString {
    let bytes = value.as_bytes();

    if !bytes.contains(&b',') && !bytes.contains(&b'\\') {
        return value.to_os_string();
    }

    let mut out = Vec::with_capacity(bytes.len() + 8);

    for &byte in bytes {
        if byte == b',' || byte == b'\\' {
            out.push(b'\\');
        }
        out.push(byte);
    }

    <OsString as std::os::unix::ffi::OsStringExt>::from_vec(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_render_as_a_comma_separated_list() {
        let opts = render_options(&[MountOpt::Ro, MountOpt::NoSuid, MountOpt::MaxRead(4096)]);

        assert_eq!(opts, OsStr::new("ro,nosuid,max_read=4096"));
    }

    #[test]
    fn no_options_render_as_nothing() {
        assert_eq!(render_options(&[]), OsStr::new(""));
    }

    // The helper splits on commas and unescapes, so a name carrying either
    // character has to arrive escaped or it would be read as two options.
    #[test]
    fn a_name_with_a_comma_or_a_backslash_is_escaped() {
        let opts = render_options(&[MountOpt::FsName(r"we,ird\name".into())]);

        assert_eq!(opts, OsStr::new(r"fsname=we\,ird\\name"));
    }

    #[test]
    fn an_ordinary_name_is_left_alone() {
        let opts = render_options(&[MountOpt::FsName("myfs".into())]);

        assert_eq!(opts, OsStr::new("fsname=myfs"));
    }

    // A name need not be UTF-8, and must not be replaced with U+FFFD.
    #[test]
    fn a_name_that_isnt_utf8_survives() {
        use std::os::unix::ffi::OsStringExt;

        let name = OsString::from_vec(vec![b'a', 0xff, b'b']);
        let opts = render_options(&[MountOpt::FsName(name)]);

        assert_eq!(opts.as_bytes(), b"fsname=a\xffb");
    }

    // Mounts for real, so it needs an installed helper and a usable
    // `/dev/fuse`. Both are a requirement for testing this crate rather than
    // something to skip over.
    //
    // Nothing may `stat` the mountpoint while this runs. No server is answering
    // `FUSE_INIT`, so any access blocks until the unmount below.
    #[test]
    fn the_device_arrives_and_the_mount_goes_away() {
        fn mounts() -> String {
            std::fs::read_to_string("/proc/self/mounts").unwrap_or_default()
        }

        let dir = std::env::temp_dir().join(format!("fuseio-mount-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let outcome = compio::runtime::Runtime::new().unwrap().block_on(async {
            let mounter = Fusermount::new();
            let (conn, pending) = mounter
                .mount(&dir, &[MountOpt::Rw, MountOpt::NoSuid], 1)
                .await?;
            let unmounter = pending.await?;

            // Read before unmounting, asserted after, so a surprise here can't
            // leave the mount behind.
            let while_mounted = mounts();

            unmounter
                .unmount(Conn::Shared(conn), &dir, &[])
                .await
                .map(|()| while_mounted)
        });

        let after = mounts();
        let _ = std::fs::remove_dir(&dir);

        let while_mounted = outcome.expect("mount and unmount");
        let needle = dir.display().to_string();

        assert!(
            while_mounted.contains(&needle),
            "not in /proc/self/mounts while mounted"
        );
        assert!(!after.contains(&needle), "still mounted afterwards");
    }

    // Runs the helper, but mounts nothing: the mountpoint doesn't exist, so the
    // helper fails before it opens the device.
    #[test]
    fn a_mountpoint_that_cannot_be_mounted_reports_the_helpers_status() {
        let err = compio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                Fusermount::new()
                    .mount(Path::new("/nonexistent/fuseio-test"), &[], 1)
                    .await
                    .map(|_| ())
            })
            .expect_err("a missing mountpoint cannot be mounted");

        // The helper's exit status, not just the socket's end-of-file, since the
        // status is what says why.
        assert!(
            err.to_string().contains("failed to mount"),
            "{err} ({:?})",
            err.kind()
        );
    }

    #[test]
    fn a_named_program_is_used_as_is() {
        let mounter = Fusermount::from_static("/nonexistent/fusermount3");

        assert_eq!(
            mounter.program().unwrap(),
            Path::new("/nonexistent/fusermount3")
        );
    }

    // Whatever this machine has, the search must find it rather than erroring,
    // and must prefer the fuse3 helper.
    #[test]
    fn the_search_finds_an_installed_helper() {
        let found = Fusermount::new()
            .program()
            .expect("testing this crate needs fusermount or fusermount3 installed");

        let name = found.file_name().unwrap();

        assert!(
            PROGRAMS.iter().any(|program| name == OsStr::new(program)),
            "{}",
            found.display()
        );
        assert!(is_executable(&found), "{}", found.display());
    }
}
