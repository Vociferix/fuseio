use std::num::NonZeroI32;

macro_rules! error {
    ($x:ident) => {
        Error(match NonZeroI32::new(nix::libc::$x) {
            Some(x) => x,
            None => panic!(),
        })
    };
    ($x:literal) => {
        Error(match NonZeroI32::new($x) {
            Some(x) => x,
            None => panic!(),
        })
    };
}

macro_rules! def_error_consts {
    ($($(#[$attrs:meta])* const $e:ident = $c:expr;)*) => {
        impl Error {
            $($(#[$attrs])* pub const $e: Error = $c;)*

            #[cfg(test)]
            const ALL: &[Error] = &[$(Self::$e),*];
        }
    }
}

/// Error type returned for FUSE filsystem operations.
///
/// This type is a wrapper around errno error codes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error(NonZeroI32);

def_error_consts! {
    /// Operation not permitted
    const EPERM = error!(EPERM);
    /// No such file or directory
    const ENOENT = error!(ENOENT);
    /// No such process
    const ESRCH = error!(ESRCH);
    /// Interrupted system call
    const EINTR = error!(EINTR);
    /// Input/output error
    const EIO = error!(EIO);
    /// No such device or address
    const ENXIO = error!(ENXIO);
    /// Argument list too long
    const E2BIG = error!(E2BIG);
    /// Exec format error
    const ENOEXEC = error!(ENOEXEC);
    /// Bad file descriptor
    const EBADF = error!(EBADF);
    /// No child processes
    const ECHILD = error!(ECHILD);
    /// Resource temporarily unavailable
    const EAGAIN = error!(EAGAIN);
    /// Cannot allocate memory
    const ENOMEM = error!(ENOMEM);
    /// Permission denied
    const EACCES = error!(EACCES);
    /// Bad address
    const EFAULT = error!(EFAULT);
    /// Block device required
    const ENOTBLK = error!(ENOTBLK);
    /// Device or resource busy
    const EBUSY = error!(EBUSY);
    /// File exists
    const EEXIST = error!(EEXIST);
    /// Invalid cross-device link
    const EXDEV = error!(EXDEV);
    /// No such device
    const ENODEV = error!(ENODEV);
    /// Not a directory
    const ENOTDIR = error!(ENOTDIR);
    /// Is a directory
    const EISDIR = error!(EISDIR);
    /// Invalid argument
    const EINVAL = error!(EINVAL);
    /// Too many open files in system
    const ENFILE = error!(ENFILE);
    /// Too many open files
    const EMFILE = error!(EMFILE);
    /// Inappropriate ioctl for device
    const ENOTTY = error!(ENOTTY);
    /// Text file busy
    const ETXTBSY = error!(ETXTBSY);
    /// File too large
    const EFBIG = error!(EFBIG);
    /// No space left on device
    const ENOSPC = error!(ENOSPC);
    /// Illegal seek
    const ESPIPE = error!(ESPIPE);
    /// Read-only file system
    const EROFS = error!(EROFS);
    /// Too many links
    const EMLINK = error!(EMLINK);
    /// Broken pipe
    const EPIPE = error!(EPIPE);
    /// Numerical argument out of domain
    const EDOM = error!(EDOM);
    /// Numerical result out of range
    const ERANGE = error!(ERANGE);
    /// Resource deadlock avoided
    const EDEADLK = error!(EDEADLK);
    /// File name too long
    const ENAMETOOLONG = error!(ENAMETOOLONG);
    /// No locks available
    const ENOLCK = error!(ENOLCK);
    /// Function not implemented
    const ENOSYS = error!(ENOSYS);
    /// Directory not empty
    const ENOTEMPTY = error!(ENOTEMPTY);
    /// Too many levels of symbolic links
    const ELOOP = error!(ELOOP);
    /// Resource temporarily unavailable
    const EWOULDBLOCK = error!(EWOULDBLOCK);
    /// No message of desired type
    const ENOMSG = error!(ENOMSG);
    /// Identifier removed
    const EIDRM = error!(EIDRM);
    /// Object is remote
    const EREMOTE = error!(EREMOTE);
    /// Link has been severed
    const ENOLINK = error!(ENOLINK);
    /// Protocol error
    const EPROTO = error!(EPROTO);
    /// Multihop attempted
    const EMULTIHOP = error!(EMULTIHOP);
    /// Bad message
    const EBADMSG = error!(EBADMSG);
    /// Value too large for defined data type
    const EOVERFLOW = error!(EOVERFLOW);
    /// Invalid or incomplete multibyte or wide character
    const EILSEQ = error!(EILSEQ);
    /// Too many users
    const EUSERS = error!(EUSERS);
    /// Socket operation on non-socket
    const ENOTSOCK = error!(ENOTSOCK);
    /// Destination address required
    const EDESTADDRREQ = error!(EDESTADDRREQ);
    /// Message too long
    const EMSGSIZE = error!(EMSGSIZE);
    /// Protocol wrong type for socket
    const EPROTOTYPE = error!(EPROTOTYPE);
    /// Protocol not available
    const ENOPROTOOPT = error!(ENOPROTOOPT);
    /// Protocol not supported
    const EPROTONOSUPPORT = error!(EPROTONOSUPPORT);
    /// Socket type not supported
    const ESOCKTNOSUPPORT = error!(ESOCKTNOSUPPORT);
    /// Operation not supported
    const EOPNOTSUPP = error!(EOPNOTSUPP);
    /// Protocol family not supported
    const EPFNOSUPPORT = error!(EPFNOSUPPORT);
    /// Address family not supported by protocol
    const EAFNOSUPPORT = error!(EAFNOSUPPORT);
    /// Address already in use
    const EADDRINUSE = error!(EADDRINUSE);
    /// Cannot assign requested address
    const EADDRNOTAVAIL = error!(EADDRNOTAVAIL);
    /// Network is down
    const ENETDOWN = error!(ENETDOWN);
    /// Network is unreachable
    const ENETUNREACH = error!(ENETUNREACH);
    /// Network dropped connection on reset
    const ENETRESET = error!(ENETRESET);
    /// Software caused connection abort
    const ECONNABORTED = error!(ECONNABORTED);
    /// Connection reset by peer
    const ECONNRESET = error!(ECONNRESET);
    /// No buffer space available
    const ENOBUFS = error!(ENOBUFS);
    /// Transport endpoint is already connected
    const EISCONN = error!(EISCONN);
    /// Transport endpoint is not connected
    const ENOTCONN = error!(ENOTCONN);
    /// Cannot send after transport endpoint shutdown
    const ESHUTDOWN = error!(ESHUTDOWN);
    /// Too many references: cannot splice
    const ETOOMANYREFS = error!(ETOOMANYREFS);
    /// Connection timed out
    const ETIMEDOUT = error!(ETIMEDOUT);
    /// Connection refused
    const ECONNREFUSED = error!(ECONNREFUSED);
    /// Host is down
    const EHOSTDOWN = error!(EHOSTDOWN);
    /// No route to host
    const EHOSTUNREACH = error!(EHOSTUNREACH);
    /// Operation already in progress
    const EALREADY = error!(EALREADY);
    /// Operation now in progress
    const EINPROGRESS = error!(EINPROGRESS);
    /// Stale file handle
    const ESTALE = error!(ESTALE);
    /// Disk quota exceeded
    const EDQUOT = error!(EDQUOT);
    /// Operation cancelled
    const ECANCELED = error!(ECANCELED);
    /// Owner died
    const EOWNERDEAD = error!(EOWNERDEAD);
    /// State not recoverable
    const ENOTRECOVERABLE = error!(ENOTRECOVERABLE);
    /// Operation not supported
    const ENOTSUP = error!(ENOTSUP);

    /// Wrong file type.
    ///
    /// On Linux, this is just an alias of [`EINVAL`](Self::EINVAL).
    #[cfg(target_os = "linux")]
    const EFTYPE = error!(EINVAL);

    /// Wrong file type.
    ///
    /// On Linux, this is just an alias of [`EINVAL`](Self::EINVAL).
    #[cfg(not(target_os = "linux"))]
    const EFTYPE = error!(EFTYPE);

    /// Extended attribute not found
    ///
    /// This is not a true errno error code name. Instead it
    /// is an alias of the proper error code for the current
    /// platform that should be returned when an xattr is not
    /// found.
    #[cfg(target_os = "linux")]
    const ENOXATTR = error!(ENODATA);

    /// Extended attribute not found
    ///
    /// This is not a true errno error code name. Instead it
    /// is an alias of the proper error code for the current
    /// platform that should be returned when an xattr is not
    /// found.
    #[cfg(not(target_os = "linux"))]
    const ENOXATTR = error!(ENOATTR);

    /// The largest errno this platform defines, and so the largest one a reply
    /// may carry.
    ///
    /// Every kernel this crate speaks to accepts at least this much, and each
    /// rejects more, some of them destructively:
    ///
    /// - Linux allows `1..=511`, and rejects anything larger *before* matching
    ///   the reply to its request, leaving the caller waiting forever
    ///   (`fs/fuse/dev.c`, `oh.error <= -512`).
    /// - FreeBSD quietly substitutes `EIO` above `ELAST`, but under the
    ///   `linux_errnos` mount option it fails the write outright for anything
    ///   past its translation table, which loses the reply the same way.
    /// - macOS performs no range check, so a larger value reaches the caller as
    ///   a meaningless errno.
    ///
    /// Bounding replies by the largest errno the platform actually defines stays
    /// inside all three, and gives up nothing: a larger value has no meaning
    /// here anyway.
    #[cfg(not(target_os = "linux"))]
    const ELAST = error!(ELAST);

    /// The largest errno this platform defines, and so the largest one a reply
    /// may carry.
    ///
    /// Every kernel this crate speaks to accepts at least this much, and each
    /// rejects more, some of them destructively:
    ///
    /// - Linux allows `1..=511`, and rejects anything larger *before* matching
    ///   the reply to its request, leaving the caller waiting forever
    ///   (`fs/fuse/dev.c`, `oh.error <= -512`).
    /// - FreeBSD quietly substitutes `EIO` above `ELAST`, but under the
    ///   `linux_errnos` mount option it fails the write outright for anything
    ///   past its translation table, which loses the reply the same way.
    /// - macOS performs no range check, so a larger value reaches the caller as
    ///   a meaningless errno.
    ///
    /// Bounding replies by the largest errno the platform actually defines stays
    /// inside all three, and gives up nothing: a larger value has no meaning
    /// here anyway.
    //
    // Linux has no `ELAST`. `EHWPOISON` is the highest errno it defines
    // (`asm-generic/errno.h`), and libc doesn't export it.
    #[cfg(target_os = "linux")]
    const ELAST = error!(133);
}

impl Error {
    pub const fn raw_os_error(&self) -> i32 {
        self.0.get()
    }

    /// Returns [`None`] unless `error` is an errno this platform defines, which
    /// is `1..=`[`ELAST`](Self::ELAST).
    pub const fn from_raw_os_error(error: i32) -> Option<Self> {
        if error < 1 || error > Self::ELAST.raw_os_error() {
            return None;
        }

        if let Some(error) = NonZeroI32::new(error) {
            Some(Self(error))
        } else {
            None
        }
    }
}

impl From<Error> for NonZeroI32 {
    fn from(err: Error) -> Self {
        err.0
    }
}

impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        Self::from_raw_os_error(err.raw_os_error())
    }
}

impl From<Error> for std::io::ErrorKind {
    fn from(err: Error) -> Self {
        std::io::Error::from(err).kind()
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        if let Some(err) = err.raw_os_error().and_then(Self::from_raw_os_error) {
            err
        } else {
            err.kind().into()
        }
    }
}

impl From<std::io::ErrorKind> for Error {
    fn from(kind: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind::*;

        match kind {
            ArgumentListTooLong => Self::E2BIG,
            AddrInUse => Self::EADDRINUSE,
            AddrNotAvailable => Self::EADDRNOTAVAIL,
            ResourceBusy => Self::EBUSY,
            ConnectionAborted => Self::ECONNABORTED,
            ConnectionRefused => Self::ECONNREFUSED,
            ConnectionReset => Self::ECONNRESET,
            Deadlock => Self::EDEADLK,
            QuotaExceeded => Self::EDQUOT,
            AlreadyExists => Self::EEXIST,
            FileTooLarge => Self::EFBIG,
            HostUnreachable => Self::EHOSTUNREACH,
            Interrupted => Self::EINTR,
            InvalidInput => Self::EINVAL,
            IsADirectory => Self::EISDIR,
            //FilesystemLoop => Self::ELOOP,
            NotFound => Self::ENOENT,
            OutOfMemory => Self::ENOMEM,
            StorageFull => Self::ENOSPC,
            Unsupported => Self::EOPNOTSUPP,
            TooManyLinks => Self::EMLINK,
            InvalidFilename => Self::ENAMETOOLONG,
            NetworkDown => Self::ENETDOWN,
            NetworkUnreachable => Self::ENETUNREACH,
            NotConnected => Self::ENOTCONN,
            NotADirectory => Self::ENOTDIR,
            BrokenPipe => Self::EPIPE,
            ReadOnlyFilesystem => Self::EROFS,
            NotSeekable => Self::ESPIPE,
            StaleNetworkFileHandle => Self::ESTALE,
            TimedOut => Self::ETIMEDOUT,
            ExecutableFileBusy => Self::ETXTBSY,
            CrossesDevices => Self::EXDEV,
            //InProgress => Self::EINPROGRESS,
            PermissionDenied => Self::EACCES,
            WouldBlock => Self::EAGAIN,
            _ => Self::EINVAL,
        }
    }
}

/// `Errno::UnknownErrno` (0) has no corresponding error and converts to
/// [`Error::EIO`].
impl From<nix::errno::Errno> for Error {
    fn from(errno: nix::errno::Errno) -> Self {
        Self::from_raw_os_error(errno as i32).unwrap_or(Self::EIO)
    }
}

/// Error codes unknown to `nix` convert to `Errno::UnknownErrno`.
impl From<Error> for nix::errno::Errno {
    fn from(err: Error) -> Self {
        Self::from_raw(err.raw_os_error())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&std::io::Error::from(*self), f)
    }
}

impl std::error::Error for Error {}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<bytemuck::PodCastError> for Error {
    fn from(err: bytemuck::PodCastError) -> Self {
        let _ = err;
        Self::EINVAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_errno_round_trips() {
        let err = Error::from_raw_os_error(Error::ENOENT.raw_os_error()).unwrap();

        assert_eq!(err, Error::ENOENT);
    }

    #[test]
    fn zero_is_not_an_error() {
        assert!(Error::from_raw_os_error(0).is_none());
    }

    #[test]
    fn errnos_this_platform_doesnt_define_are_rejected() {
        assert!(Error::from_raw_os_error(-5).is_none());
        assert!(Error::from_raw_os_error(Error::ELAST.raw_os_error() + 1).is_none());
        assert!(Error::from_raw_os_error(9999).is_none());
    }

    #[test]
    fn the_highest_accepted_errno_is_allowed() {
        assert!(Error::from_raw_os_error(Error::ELAST.raw_os_error()).is_some());
    }

    // Nothing rewrites an out-of-range errno on the way out any more, so every
    // named one has to be within what this platform's kernel accepts. The
    // `error!` macro only checks for zero, so this is the only thing that would
    // catch a constant above the ceiling.
    #[test]
    fn every_named_errno_is_within_the_limit() {
        for err in Error::ALL.iter().copied() {
            assert!(err <= Error::ELAST, "{err:?} exceeds {:?}", Error::ELAST);
        }
    }

    #[test]
    fn an_io_error_outside_the_limit_falls_back_to_its_kind() {
        let io = std::io::Error::from_raw_os_error(9999);
        let err = Error::from(io);

        assert!((1..=Error::ELAST.raw_os_error()).contains(&err.raw_os_error()));
    }

    // `open_passthrough` documents one `ENOTSUP` for both "this connection has no
    // passthrough" and the kernel's `EOPNOTSUPP` for a build without
    // `CONFIG_FUSE_PASSTHROUGH`, which holds only while the two are the same
    // errno. They differ on macOS, where passthrough doesn't exist anyway.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_unsupported_operation_has_one_errno() {
        assert_eq!(nix::libc::ENOTSUP, nix::libc::EOPNOTSUPP);
        assert_eq!(Error::ENOTSUP.raw_os_error(), nix::libc::EOPNOTSUPP);
    }
}
