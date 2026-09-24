//! Conversions for the protocol's timestamps.
//!
//! Every timestamp is a count of seconds since the epoch plus a nanosecond
//! remainder in `0..1_000_000_000`, following the `timespec` convention: the
//! remainder always moves forward in time, so one half-second before the epoch
//! is `-1` seconds plus 500 ms, not `0` seconds minus 500 ms.
//!
//! The seconds are signed, but most requests and replies carry them in a `u64`
//! field, so they arrive and leave as two's-complement bits.

use crate::{Error, Result};

use std::time::{Duration, SystemTime};

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Splits a time into the seconds and nanoseconds to put on the wire.
pub(crate) fn split(time: SystemTime) -> (i64, u32) {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since) => (
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
            since.subsec_nanos(),
        ),
        Err(err) => {
            let before = err.duration();
            let secs = i64::try_from(before.as_secs()).unwrap_or(i64::MAX);
            let nanos = before.subsec_nanos();

            // Carry the remainder forward, so the nanoseconds stay positive.
            if nanos == 0 {
                (-secs, 0)
            } else {
                (-secs - 1, NANOS_PER_SEC as u32 - nanos)
            }
        }
    }
}

/// Splits a time into the seconds and nanoseconds to put on the wire, with the
/// seconds as the bits of the `u64` field that holds them.
pub(crate) fn split_raw(time: SystemTime) -> (u64, u32) {
    let (secs, nanos) = split(time);

    (secs.cast_unsigned(), nanos)
}

/// Rebuilds a time from the seconds and nanoseconds on the wire.
///
/// The seconds are the bits of the field that holds them, which is unsigned in
/// most requests but always carries a signed count.
pub(crate) fn join_raw(secs: u64, nanos: u32) -> Result<SystemTime> {
    join(secs.cast_signed(), nanos)
}

/// Rebuilds a time from the seconds and nanoseconds on the wire.
pub(crate) fn join(secs: i64, nanos: u32) -> Result<SystemTime> {
    if u64::from(nanos) >= NANOS_PER_SEC {
        return Err(Error::EINVAL);
    }

    let nanos = Duration::from_nanos(nanos.into());

    let time = if secs >= 0 {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(secs.cast_unsigned()))
            .and_then(|time| time.checked_add(nanos))
    } else {
        // `-secs` would overflow for `i64::MIN`, so negate as unsigned.
        let before = Duration::from_secs(secs.unsigned_abs());

        SystemTime::UNIX_EPOCH
            .checked_sub(before)
            .and_then(|time| time.checked_add(nanos))
    };

    time.ok_or(Error::EINVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64, nanos: u32) -> SystemTime {
        join(secs, nanos).unwrap()
    }

    #[test]
    fn the_epoch_round_trips() {
        assert_eq!(split(at(0, 0)), (0, 0));
    }

    #[test]
    fn a_later_time_round_trips() {
        assert_eq!(
            split(at(1_700_000_000, 123_456_789)),
            (1_700_000_000, 123_456_789)
        );
    }

    #[test]
    fn a_whole_second_before_the_epoch_round_trips() {
        assert_eq!(split(at(-1, 0)), (-1, 0));
        assert_eq!(split(at(-86_400, 0)), (-86_400, 0));
    }

    #[test]
    fn a_fraction_before_the_epoch_round_trips() {
        // Half a second before the epoch, which the wire spells as -1 seconds
        // plus 500 ms rather than 0 seconds minus 500 ms.
        assert_eq!(split(at(-1, 500_000_000)), (-1, 500_000_000));
        assert_eq!(split(at(-2, 1)), (-2, 1));
    }

    #[test]
    fn a_time_before_the_epoch_is_before_it() {
        assert!(at(-1, 999_999_999) < SystemTime::UNIX_EPOCH);
        assert!(at(-1, 500_000_000) < at(0, 0));
        assert!(at(-2, 0) < at(-1, 0));
    }

    #[test]
    fn negative_seconds_travel_as_twos_complement() {
        let (raw, nanos) = split_raw(at(-1, 500_000_000));

        assert_eq!(raw, (-1i64).cast_unsigned());
        assert_eq!(nanos, 500_000_000);
        assert_eq!(join_raw(raw, nanos).unwrap(), at(-1, 500_000_000));
    }

    #[test]
    fn an_out_of_range_remainder_is_rejected() {
        assert_eq!(join(0, 1_000_000_000).unwrap_err(), Error::EINVAL);
        assert_eq!(join(-1, u32::MAX).unwrap_err(), Error::EINVAL);
    }

    #[test]
    fn extreme_seconds_dont_panic() {
        // Whatever the platform's `SystemTime` can hold, these either convert or
        // report EINVAL.
        let _ = join(i64::MIN, 0);
        let _ = join(i64::MAX, 999_999_999);
        let _ = join_raw(u64::MAX, 0);
    }
}
