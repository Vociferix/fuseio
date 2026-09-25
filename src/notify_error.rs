use crate::Error;

/// Why a notification to the kernel didn't happen.
///
/// This is deliberately not an [`Error`], so a notification's failure can't be
/// returned from an operation by mistake: an errno from a notification would
/// become the reply to whatever the filesystem was doing, and a kernel that sees
/// `ENOSYS` stops sending that operation for the rest of the mount.
#[derive(Debug)]
pub enum NotifyError {
    /// The kernel doesn't support this notification, because it is older than
    /// the version that added it or didn't negotiate the feature.
    Unsupported,

    /// The kernel has nothing cached to act on, which is the usual answer for an
    /// inode or entry it never looked up, and is normally not worth reporting.
    NotCached,

    /// The notification couldn't be sent.
    Io(std::io::Error),
}

impl NotifyError {
    /// Whether this only means the kernel had nothing to do.
    pub fn is_not_cached(&self) -> bool {
        matches!(self, Self::NotCached)
    }
}

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("the kernel does not support this notification"),
            Self::NotCached => f.write_str("the kernel has nothing cached"),
            Self::Io(err) => write!(f, "failed to notify the kernel: {err}"),
        }
    }
}

impl std::error::Error for NotifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NotifyError {
    fn from(err: std::io::Error) -> Self {
        match err.raw_os_error().and_then(Error::from_raw_os_error) {
            Some(Error::ENOENT) => Self::NotCached,
            Some(Error::ENOSYS | Error::ENOTSUP) => Self::Unsupported,
            _ => Self::Io(err),
        }
    }
}

impl From<Error> for NotifyError {
    fn from(err: Error) -> Self {
        match err {
            Error::ENOENT => Self::NotCached,
            Error::ENOSYS | Error::ENOTSUP => Self::Unsupported,
            err => Self::Io(err.into()),
        }
    }
}

impl From<std::convert::Infallible> for NotifyError {
    fn from(err: std::convert::Infallible) -> Self {
        match err {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_uncached_inode_is_not_a_failure() {
        let err = NotifyError::from(Error::ENOENT);

        assert!(err.is_not_cached());
    }

    #[test]
    fn an_old_kernel_is_unsupported() {
        for err in [Error::ENOSYS, Error::ENOTSUP] {
            assert!(matches!(NotifyError::from(err), NotifyError::Unsupported));
        }
    }

    #[test]
    fn anything_else_is_an_io_failure() {
        let err = NotifyError::from(Error::EIO);

        assert!(matches!(err, NotifyError::Io(_)));
        assert!(!err.is_not_cached());
    }

    #[test]
    fn an_io_error_keeps_its_meaning() {
        let err = NotifyError::from(std::io::Error::from_raw_os_error(
            Error::ENOENT.raw_os_error(),
        ));

        assert!(err.is_not_cached());
    }
}
