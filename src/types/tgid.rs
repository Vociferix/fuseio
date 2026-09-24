use std::num::NonZeroI32;

/// The identifier of a thread group, which is what a process id names.
///
/// This is deliberately distinct from [`Pid`](super::Pid), which a request
/// header carries: Linux puts a thread's own id there and a thread group's id in
/// a lock, so the two can differ for a threaded caller. The BSDs and macOS put a
/// process id in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tgid(NonZeroI32);

impl Tgid {
    /// Returns [`None`] for zero, which a kernel sends when no thread group
    /// owns the lock, as on an unlock.
    pub const fn from_raw(tgid: i32) -> Option<Self> {
        if let Some(tgid) = NonZeroI32::new(tgid) {
            Some(Self(tgid))
        } else {
            None
        }
    }

    pub const fn as_raw(self) -> i32 {
        self.0.get()
    }
}

impl From<Tgid> for nix::unistd::Pid {
    fn from(tgid: Tgid) -> Self {
        Self::from_raw(tgid.as_raw())
    }
}

impl From<NonZeroI32> for Tgid {
    fn from(tgid: NonZeroI32) -> Self {
        Self(tgid)
    }
}

impl From<Tgid> for NonZeroI32 {
    fn from(tgid: Tgid) -> Self {
        tgid.0
    }
}

impl std::fmt::Display for Tgid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_group_round_trips() {
        assert_eq!(Tgid::from_raw(4321).unwrap().as_raw(), 4321);
    }

    #[test]
    fn no_thread_group_is_none() {
        assert!(Tgid::from_raw(0).is_none());
    }

    #[test]
    fn it_converts_to_a_pid() {
        let tgid = Tgid::from_raw(4321).unwrap();

        assert_eq!(nix::unistd::Pid::from(tgid).as_raw(), 4321);
    }
}
