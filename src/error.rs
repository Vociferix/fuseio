use std::num::NonZeroI32;

macro_rules! error {
    ($x:ident) => {
        Error(match NonZeroI32::new(nix::libc::$x) {
            Some(x) => x,
            None => panic!(),
        })
    };
}

/// Error type returned for FUSE filsystem operations.
///
/// This type is a wrapper around errno error codes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error(NonZeroI32);

impl Error {
    /// Operation not permitted
    pub const EPERM: Error = error!(EPERM);
    /// No such file or directory
    pub const ENOENT: Error = error!(ENOENT);
    /// No such process
    pub const ESRCH: Error = error!(ESRCH);
    /// Interrupted system call
    pub const EINTR: Error = error!(EINTR);
    /// Input/output error
    pub const EIO: Error = error!(EIO);
    /// No such device or address
    pub const ENXIO: Error = error!(ENXIO);
    /// Argument list too long
    pub const E2BIG: Error = error!(E2BIG);
    /// Exec format error
    pub const ENOEXEC: Error = error!(ENOEXEC);
    /// Bad file descriptor
    pub const EBADF: Error = error!(EBADF);
    /// No child processes
    pub const ECHILD: Error = error!(ECHILD);
    /// Resource temporarily unavailable
    pub const EAGAIN: Error = error!(EAGAIN);
    /// Cannot allocate memory
    pub const ENOMEM: Error = error!(ENOMEM);
    /// Permission denied
    pub const EACCES: Error = error!(EACCES);
    /// Bad address
    pub const EFAULT: Error = error!(EFAULT);
    /// Block device required
    pub const ENOTBLK: Error = error!(ENOTBLK);
    /// Device or resource busy
    pub const EBUSY: Error = error!(EBUSY);
    /// File exists
    pub const EEXIST: Error = error!(EEXIST);
    /// Invalid cross-device link
    pub const EXDEV: Error = error!(EXDEV);
    /// No such device
    pub const ENODEV: Error = error!(ENODEV);
    /// Not a directory
    pub const ENOTDIR: Error = error!(ENOTDIR);
    /// Is a directory
    pub const EISDIR: Error = error!(EISDIR);
    /// Invalid argument
    pub const EINVAL: Error = error!(EINVAL);
    /// Too many open files in system
    pub const ENFILE: Error = error!(ENFILE);
    /// Too many open files
    pub const EMFILE: Error = error!(EMFILE);
    /// Inappropriate ioctl for device
    pub const ENOTTY: Error = error!(ENOTTY);
    /// Text file busy
    pub const ETXTBSY: Error = error!(ETXTBSY);
    /// File too large
    pub const EFBIG: Error = error!(EFBIG);
    /// No space left on device
    pub const ENOSPC: Error = error!(ENOSPC);
    /// Illegal seek
    pub const ESPIPE: Error = error!(ESPIPE);
    /// Read-only file system
    pub const EROFS: Error = error!(EROFS);
    /// Too many links
    pub const EMLINK: Error = error!(EMLINK);
    /// Broken pipe
    pub const EPIPE: Error = error!(EPIPE);
    /// Numerical argument out of domain
    pub const EDOM: Error = error!(EDOM);
    /// Numerical result out of range
    pub const ERANGE: Error = error!(ERANGE);
    /// Resource deadlock avoided
    pub const EDEADLK: Error = error!(EDEADLK);
    /// File name too long
    pub const ENAMETOOLONG: Error = error!(ENAMETOOLONG);
    /// No locks available
    pub const ENOLCK: Error = error!(ENOLCK);
    /// Function not implemented
    pub const ENOSYS: Error = error!(ENOSYS);
    /// Directory not empty
    pub const ENOTEMPTY: Error = error!(ENOTEMPTY);
    /// Too many levels of symbolic links
    pub const ELOOP: Error = error!(ELOOP);
    /// Resource temporarily unavailable
    pub const EWOULDBLOCK: Error = error!(EWOULDBLOCK);
    /// No message of desired type
    pub const ENOMSG: Error = error!(ENOMSG);
    /// Identifier removed
    pub const EIDRM: Error = error!(EIDRM);
    /// Object is remote
    pub const EREMOTE: Error = error!(EREMOTE);
    /// Link has been severed
    pub const ENOLINK: Error = error!(ENOLINK);
    /// Protocol error
    pub const EPROTO: Error = error!(EPROTO);
    /// Multihop attempted
    pub const EMULTIHOP: Error = error!(EMULTIHOP);
    /// Bad message
    pub const EBADMSG: Error = error!(EBADMSG);
    /// Value too large for defined data type
    pub const EOVERFLOW: Error = error!(EOVERFLOW);
    /// Invalid or incomplete multibyte or wide character
    pub const EILSEQ: Error = error!(EILSEQ);
    /// Too many users
    pub const EUSERS: Error = error!(EUSERS);
    /// Socket operation on non-socket
    pub const ENOTSOCK: Error = error!(ENOTSOCK);
    /// Destination address required
    pub const EDESTADDRREQ: Error = error!(EDESTADDRREQ);
    /// Message too long
    pub const EMSGSIZE: Error = error!(EMSGSIZE);
    /// Protocol wrong type for socket
    pub const EPROTOTYPE: Error = error!(EPROTOTYPE);
    /// Protocol not available
    pub const ENOPROTOOPT: Error = error!(ENOPROTOOPT);
    /// Protocol not supported
    pub const EPROTONOSUPPORT: Error = error!(EPROTONOSUPPORT);
    /// Socket type not supported
    pub const ESOCKTNOSUPPORT: Error = error!(ESOCKTNOSUPPORT);
    /// Operation not supported
    pub const EOPNOTSUPP: Error = error!(EOPNOTSUPP);
    /// Protocol family not supported
    pub const EPFNOSUPPORT: Error = error!(EPFNOSUPPORT);
    /// Address family not supported by protocol
    pub const EAFNOSUPPORT: Error = error!(EAFNOSUPPORT);
    /// Address already in use
    pub const EADDRINUSE: Error = error!(EADDRINUSE);
    /// Cannot assign requested address
    pub const EADDRNOTAVAIL: Error = error!(EADDRNOTAVAIL);
    /// Network is down
    pub const ENETDOWN: Error = error!(ENETDOWN);
    /// Network is unreachable
    pub const ENETUNREACH: Error = error!(ENETUNREACH);
    /// Network dropped connection on reset
    pub const ENETRESET: Error = error!(ENETRESET);
    /// Software caused connection abort
    pub const ECONNABORTED: Error = error!(ECONNABORTED);
    /// Connection reset by peer
    pub const ECONNRESET: Error = error!(ECONNRESET);
    /// No buffer space available
    pub const ENOBUFS: Error = error!(ENOBUFS);
    /// Transport endpoint is already connected
    pub const EISCONN: Error = error!(EISCONN);
    /// Transport endpoint is not connected
    pub const ENOTCONN: Error = error!(ENOTCONN);
    /// Cannot send after transport endpoint shutdown
    pub const ESHUTDOWN: Error = error!(ESHUTDOWN);
    /// Too many references: cannot splice
    pub const ETOOMANYREFS: Error = error!(ETOOMANYREFS);
    /// Connection timed out
    pub const ETIMEDOUT: Error = error!(ETIMEDOUT);
    /// Connection refused
    pub const ECONNREFUSED: Error = error!(ECONNREFUSED);
    /// Host is down
    pub const EHOSTDOWN: Error = error!(EHOSTDOWN);
    /// No route to host
    pub const EHOSTUNREACH: Error = error!(EHOSTUNREACH);
    /// Operation already in progress
    pub const EALREADY: Error = error!(EALREADY);
    /// Operation now in progress
    pub const EINPROGRESS: Error = error!(EINPROGRESS);
    /// Stale file handle
    pub const ESTALE: Error = error!(ESTALE);
    /// Disk quota exceeded
    pub const EDQUOT: Error = error!(EDQUOT);
    /// Operation cancelled
    pub const ECANCELED: Error = error!(ECANCELED);
    /// Owner died
    pub const EOWNERDEAD: Error = error!(EOWNERDEAD);
    /// State not recoverable
    pub const ENOTRECOVERABLE: Error = error!(ENOTRECOVERABLE);
    /// Operation not supported
    pub const ENOTSUP: Error = error!(ENOTSUP);

    /// Wrong file type.
    ///
    /// On Linux, this is just an alias of [`EINVAL`](Self::EINVAL).
    #[cfg(target_os = "linux")]
    pub const EFTYPE: Error = error!(EINVAL);

    /// Wrong file type.
    ///
    /// On Linux, this is just an alias of [`EINVAL`](Self::EINVAL).
    #[cfg(not(target_os = "linux"))]
    pub const EFTYPE: Error = error!(EFTYPE);

    /// Extended attribute not found
    ///
    /// This is not a true errno error code name. Instead it
    /// is an alias of the proper error code for the current
    /// platform that should be returned when an xattr is not
    /// found.
    #[cfg(target_os = "linux")]
    pub const ENOXATTR: Error = error!(ENODATA);

    /// Extended attribute not found
    ///
    /// This is not a true errno error code name. Instead it
    /// is an alias of the proper error code for the current
    /// platform that should be returned when an xattr is not
    /// found.
    #[cfg(not(target_os = "linux"))]
    pub const ENOXATTR: Error = error!(ENOATTR);

    pub const fn raw_os_error(&self) -> i32 {
        self.0.get()
    }

    pub const fn from_raw_os_error(error: i32) -> Option<Self> {
        if let Some(error) = NonZeroI32::new(error) {
            Some(Self(error))
        } else {
            None
        }
    }
}

impl From<NonZeroI32> for Error {
    fn from(err: NonZeroI32) -> Self {
        Self(err)
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
