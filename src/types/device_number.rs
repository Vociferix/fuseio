/// The major and minor numbers of a device node.
///
/// This holds the value the protocol carries, so it can sit directly in a
/// request or reply, and splits it only when asked. Each platform packs the two
/// halves differently, which is why they are read through
/// [`major`](Self::major) and [`minor`](Self::minor) rather than by hand.
///
/// `fuse_statx` is the exception: it carries the halves separately, so there the
/// value is split when the reply is built.
// TODO(e2e): the packing each kernel uses is taken from its own headers; verify
// once end-to-end tests can be done, especially on macOS, whose FUSE kernel
// extension is no longer open source.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DeviceNumber(u32);

const _: () = assert!(std::mem::size_of::<DeviceNumber>() == std::mem::size_of::<u32>());

impl DeviceNumber {
    /// Returns [`None`] for a pair this platform can't carry.
    ///
    /// The field is narrower than two whole numbers, and each platform divides
    /// it differently: Linux allows a major below 4096 and a minor below
    /// 1048576, macOS a major below 256 and a minor below 16777216, and FreeBSD
    /// a major below 256 and a minor whose second byte is clear. Rather than
    /// state those as ranges, a pair is accepted when it survives being packed
    /// and read back.
    pub const fn new(major: u32, minor: u32) -> Option<Self> {
        let device = Self(pack(major, minor));

        if device.major() == major && device.minor() == minor {
            Some(device)
        } else {
            None
        }
    }

    /// The device's major number, which says what driver owns it.
    pub const fn major(self) -> u32 {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            // `new_decode_dev`: 12 bits, above the low byte of the minor.
            (self.0 & 0xfff00) >> 8
        }

        #[cfg(target_vendor = "apple")]
        {
            (self.0 >> 24) & 0xff
        }

        #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
        {
            // The low half of a FreeBSD `dev_t`.
            (self.0 >> 8) & 0xff
        }
    }

    /// The device's minor number, which says which device the driver owns it is.
    pub const fn minor(self) -> u32 {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            // Split either side of the major.
            (self.0 & 0xff) | ((self.0 >> 12) & 0xfff00)
        }

        #[cfg(target_vendor = "apple")]
        {
            self.0 & 0xffffff
        }

        #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
        {
            // Also split either side of the major.
            self.0 & 0xffff_00ff
        }
    }

    /// Takes the value a FUSE request or reply carries.
    ///
    /// Every platform packs this field the way the low half of its own `dev_t` is
    /// packed, so for any device this field can carry the two agree. They are
    /// still different widths: `dev_t` is 64 bits on Linux and FreeBSD, and a
    /// device whose numbers reach beyond this field has bits that don't fit.
    /// Prefer [`from_dev`](Self::from_dev) for a number from `stat(2)`,
    /// `mknod(2)` or any other system call, since it refuses such a device
    /// instead of dropping those bits.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The value to put in a FUSE request or reply.
    ///
    /// See [`from_raw`](Self::from_raw) for how this relates to the host's
    /// `dev_t`.
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Takes the major and minor numbers from a host `dev_t`, as `stat(2)` and
    /// `mknod(2)` use.
    ///
    /// Returns [`None`] for a device the protocol's narrower field can't carry.
    // TODO(e2e): assumes each host packs `dev_t` as its headers describe; verify
    // once end-to-end tests can be done.
    // `dev_t` differs in width by platform.
    #[allow(clippy::unnecessary_cast)]
    pub fn from_dev(dev: nix::libc::dev_t) -> Option<Self> {
        #[cfg(target_os = "linux")]
        let (major, minor) = (
            nix::sys::stat::major(dev) as u32,
            nix::sys::stat::minor(dev) as u32,
        );

        #[cfg(target_vendor = "apple")]
        let (major, minor) = {
            let dev = dev as u32;

            ((dev >> 24) & 0xff, dev & 0xffffff)
        };

        // FreeBSD's `dev_t` holds the bytes as `MMMmmmMm`.
        #[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
        let (major, minor) = {
            let dev = dev as u64;

            (
                (((dev >> 32) & 0xffff_ff00) | ((dev >> 8) & 0xff)) as u32,
                (((dev >> 24) & 0xff00) | (dev & 0xffff_00ff)) as u32,
            )
        };

        Self::new(major, minor)
    }

    /// Builds the host `dev_t` for this device.
    // TODO(e2e): assumes each host packs `dev_t` as its headers describe; verify
    // once end-to-end tests can be done.
    pub fn as_dev(self) -> nix::libc::dev_t {
        let (major, minor) = (self.major(), self.minor());

        #[cfg(target_os = "linux")]
        {
            nix::sys::stat::makedev(u64::from(major), u64::from(minor))
        }

        #[cfg(target_vendor = "apple")]
        {
            (((major & 0xff) << 24) | (minor & 0xffffff)) as nix::libc::dev_t
        }

        #[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
        {
            let (major, minor) = (u64::from(major), u64::from(minor));

            (((major & 0xffff_ff00) << 32)
                | ((major & 0xff) << 8)
                | ((minor & 0xff00) << 24)
                | (minor & 0xffff_00ff)) as nix::libc::dev_t
        }
    }
}

/// Packs the two halves the way this platform's field expects.
const fn pack(major: u32, minor: u32) -> u32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // `new_encode_dev`.
        (minor & 0xff) | (major << 8) | ((minor & !0xff) << 12)
    }

    #[cfg(target_vendor = "apple")]
    {
        ((major & 0xff) << 24) | (minor & 0xffffff)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    {
        ((major & 0xff) << 8) | (minor & 0xffff_00ff)
    }
}

impl std::fmt::Debug for DeviceNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceNumber")
            .field("major", &self.major())
            .field("minor", &self.minor())
            .finish()
    }
}

impl std::fmt::Display for DeviceNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.major(), self.minor())
    }
}

// By the numbers rather than the packing, which interleaves them.
impl Ord for DeviceNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major(), self.minor()).cmp(&(other.major(), other.minor()))
    }
}

impl PartialOrd for DeviceNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<(u32, u32)> for DeviceNumber {
    type Error = crate::Error;

    fn try_from((major, minor): (u32, u32)) -> Result<Self, Self::Error> {
        Self::new(major, minor).ok_or(crate::Error::EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_keeps_its_numbers() {
        let dev = DeviceNumber::new(8, 3).unwrap();

        assert_eq!(dev.major(), 8);
        assert_eq!(dev.minor(), 3);
    }

    #[test]
    fn an_everyday_device_round_trips() {
        // /dev/sda3, /dev/null and /dev/tty0.
        for (major, minor) in [(8, 3), (1, 3), (4, 0)] {
            let dev = DeviceNumber::new(major, minor).unwrap();

            assert_eq!(DeviceNumber::from_raw(dev.as_raw()), dev, "{dev}");
        }
    }

    #[test]
    fn a_large_minor_round_trips() {
        // Above the 8 bits the low half of the field holds.
        for minor in [256, 4095, 65_535] {
            let dev = DeviceNumber::new(1, minor).unwrap();

            assert_eq!(DeviceNumber::from_raw(dev.as_raw()), dev, "{dev}");
        }
    }

    #[test]
    fn a_large_major_round_trips() {
        let dev = DeviceNumber::new(255, 1).unwrap();

        assert_eq!(DeviceNumber::from_raw(dev.as_raw()), dev);
    }

    #[test]
    fn zero_is_no_device() {
        let dev = DeviceNumber::from_raw(0);

        assert_eq!(dev, DeviceNumber::new(0, 0).unwrap());
        assert_eq!(dev.as_raw(), 0);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn the_packing_matches_the_kernels() {
        // `new_encode_dev(MKDEV(8, 3))`.
        assert_eq!(DeviceNumber::new(8, 3).unwrap().as_raw(), 0x803);
        assert_eq!(
            DeviceNumber::from_raw(0x803),
            DeviceNumber::new(8, 3).unwrap()
        );

        // A minor that needs the bits above the major.
        assert_eq!(DeviceNumber::new(1, 0x123).unwrap().as_raw(), 0x10_0123);
        assert_eq!(
            DeviceNumber::from_raw(0x10_0123),
            DeviceNumber::new(1, 0x123).unwrap()
        );
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn the_packing_matches_the_kernels() {
        assert_eq!(DeviceNumber::new(8, 3).unwrap().as_raw(), (8 << 24) | 3);
        assert_eq!(
            DeviceNumber::from_raw((8 << 24) | 3),
            DeviceNumber::new(8, 3).unwrap()
        );
    }

    #[test]
    fn a_major_too_large_for_the_field_is_refused() {
        // Would otherwise set bits the minor owns.
        assert!(DeviceNumber::new(0x1_0000, 0).is_none());
        assert!(DeviceNumber::new(u32::MAX, 0).is_none());
    }

    #[test]
    fn a_minor_too_large_for_the_field_is_refused() {
        assert!(DeviceNumber::new(0, u32::MAX).is_none());
    }

    #[test]
    fn every_accepted_device_survives_the_wire() {
        // Whatever each platform allows, it must read back unchanged.
        for major in [0, 1, 8, 255] {
            for minor in [0, 1, 255, 4095, 65_535] {
                if let Some(dev) = DeviceNumber::new(major, minor) {
                    assert_eq!(DeviceNumber::from_raw(dev.as_raw()), dev, "{dev}");
                }
            }
        }
    }

    #[test]
    fn the_pair_conversion_reports_a_bad_device() {
        assert_eq!(
            DeviceNumber::try_from((8, 3)).unwrap(),
            DeviceNumber::new(8, 3).unwrap()
        );
        assert_eq!(
            DeviceNumber::try_from((0, u32::MAX)).unwrap_err(),
            crate::Error::EINVAL
        );
    }

    #[test]
    fn a_host_device_round_trips() {
        for (major, minor) in [(8, 3), (1, 3), (4, 0), (0, 0)] {
            let dev = DeviceNumber::new(major, minor).unwrap();

            assert_eq!(DeviceNumber::from_dev(dev.as_dev()), Some(dev), "{dev}");
        }
    }

    // The protocol's field is narrower than a `dev_t`, so not every device fits.
    #[test]
    fn a_host_device_too_large_for_the_field_is_refused() {
        let huge = DeviceNumber::new(1, 1).unwrap().as_dev();
        let huge = huge | (0xffff_u64 as nix::libc::dev_t) << 40;

        assert!(DeviceNumber::from_dev(huge).is_none());
    }

    // Each platform packs this field the way the low half of its own `dev_t` is
    // packed, so anything the field can carry reads the same either way.
    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn the_wire_value_is_the_low_half_of_a_host_device() {
        for major in [0, 1, 8, 255, 4095] {
            for minor in [0, 1, 255, 4095, 65_535, 1_048_575] {
                let Some(dev) = DeviceNumber::new(major, minor) else {
                    continue;
                };

                assert_eq!(
                    u64::from(dev.as_raw()),
                    dev.as_dev() as u64 & 0xffff_ffff,
                    "{dev}"
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_host_device_matches_nix() {
        let dev = nix::sys::stat::makedev(8, 3);

        assert_eq!(
            DeviceNumber::from_dev(dev),
            Some(DeviceNumber::new(8, 3).unwrap())
        );
    }

    // The point of holding the packed value: it can sit in a wire struct.
    #[test]
    fn it_is_just_the_packed_value() {
        assert_eq!(size_of::<DeviceNumber>(), size_of::<u32>());
        assert_eq!(align_of::<DeviceNumber>(), align_of::<u32>());
        assert_eq!(DeviceNumber::from_raw(0x803).as_raw(), 0x803);
    }

    #[test]
    fn devices_sort_by_their_numbers() {
        let mut devices = [
            DeviceNumber::new(8, 3).unwrap(),
            DeviceNumber::new(1, 9).unwrap(),
            DeviceNumber::new(8, 1).unwrap(),
        ];

        devices.sort();

        assert_eq!(
            devices.map(|dev| (dev.major(), dev.minor())),
            [(1, 9), (8, 1), (8, 3)]
        );
    }

    #[test]
    fn a_device_reads_as_its_numbers() {
        let dev = DeviceNumber::new(8, 3).unwrap();

        assert_eq!(dev.to_string(), "8:3");
        assert_eq!(format!("{dev:?}"), "DeviceNumber { major: 8, minor: 3 }");
    }
}
