use super::{Cfg, Ino, RawHeader};
use crate::types::MonitorFlags;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Monitor {
    ino: Ino,
    flags: MonitorFlags,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    flags: u32,
    _unused: u32,
}

impl Monitor {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn begin(&self) -> bool {
        self.flags.contains(MonitorFlags::BEGIN)
    }

    pub fn end(&self) -> bool {
        self.flags.contains(MonitorFlags::END)
    }
}

impl Monitor {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            flags: MonitorFlags::from_bits_retain(raw.flags),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// macOS `FUSE_MONITOR`: a paired begin and end, with nothing else in it.
    fn body(flags: u32) -> Vec<u8> {
        let mut body = flags.to_ne_bytes().to_vec();
        body.extend_from_slice(&0xdead_beefu32.to_ne_bytes()); // padding
        body
    }

    #[test]
    fn a_begin_and_an_end_are_told_apart() {
        let begin =
            Monitor::decode(request(Opcode::MONITOR, &body(1)), root(), latest()).expect("decodes");

        assert!(begin.begin() && !begin.end());

        let end =
            Monitor::decode(request(Opcode::MONITOR, &body(2)), root(), latest()).expect("decodes");

        assert!(end.end() && !end.begin());
        assert_eq!(end.ino(), root().unwrap());
    }

    // macFUSE documents these as a matched pair, one at a time, so neither bit
    // or both is something its kext does not send. They are passed through
    // rather than refused all the same: the request takes no reply, so there is
    // nothing to refuse it with, and the two predicates answering
    // independently means an unexpected combination does nothing rather than
    // something wrong.
    #[test]
    fn a_combination_the_kext_never_sends_is_passed_through() {
        let neither =
            Monitor::decode(request(Opcode::MONITOR, &body(0)), root(), latest()).expect("decodes");

        assert!(!neither.begin() && !neither.end(), "nothing to act on");

        let both =
            Monitor::decode(request(Opcode::MONITOR, &body(3)), root(), latest()).expect("decodes");

        assert!(both.begin() && both.end(), "each bit answers for itself");
    }

    // Bits macFUSE has not defined yet are kept rather than dropped, so a
    // newer kext is not misread as sending something it did not.
    #[test]
    fn an_unknown_bit_does_not_become_a_known_one() {
        let req = Monitor::decode(request(Opcode::MONITOR, &body(1 << 5)), root(), latest())
            .expect("decodes");

        assert!(!req.begin() && !req.end());
    }
}
