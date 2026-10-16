use super::{Cfg, Ino, RawHeader};
use crate::types::ForgetIno;
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

#[derive(Debug)]
pub struct BatchForget {
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    count: u32,
    _unused: u32,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct ForgetOne {
    nodeid: u64,
    nlookup: u64,
}

const _: () = {
    assert!(std::mem::size_of::<ForgetOne>() == std::mem::size_of::<ForgetIno>());
    assert!(std::mem::align_of::<ForgetOne>() == std::mem::align_of::<ForgetIno>());
    assert!(std::mem::size_of::<Raw>().is_multiple_of(crate::buf::ALIGN));
};

impl BatchForget {
    pub fn inos(&self) -> &[ForgetIno] {
        let len = self.buf.len() / std::mem::size_of::<ForgetIno>();
        // SAFETY: The view starts `size_of::<Raw>()` bytes into a buffer that
        //         is `ALIGN`-aligned, and the assertion above pins that size to
        //         a multiple of `ALIGN`, so the pointer is aligned for
        //         `ForgetIno`. `len` is rounded down, so the slice stays inside
        //         the view, and `decode` rejected any entry with a zero
        //         `nodeid`, which is what `ForgetIno::ino` relies on.
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr().cast(), len) }
    }
}

impl BatchForget {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, forgets)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let count = raw.count as usize;

        let Some((forgets, _)) = forgets.split_slice::<ForgetOne>(count) else {
            return Err(Error::EPROTO);
        };

        if forgets.iter().any(|forget| forget.nodeid == 0) {
            return Err(Error::EINVAL);
        }

        let Some(buf) = buf.try_into_view(
            std::mem::size_of::<Raw>()
                ..(std::mem::size_of::<Raw>() + count * std::mem::size_of::<ForgetOne>()),
        ) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request};

    /// `fuse_batch_forget_in { count, dummy }`, then `count` of
    /// `fuse_forget_one { nodeid, nlookup }`.
    fn body(count: u32, entries: &[(u64, u64)]) -> Vec<u8> {
        let mut body = count.to_ne_bytes().to_vec();
        body.extend_from_slice(&0u32.to_ne_bytes()); // dummy

        for &(nodeid, nlookup) in entries {
            body.extend_from_slice(&nodeid.to_ne_bytes());
            body.extend_from_slice(&nlookup.to_ne_bytes());
        }

        body
    }

    fn decode(count: u32, entries: &[(u64, u64)]) -> Result<BatchForget> {
        BatchForget::decode(
            request(Opcode::BATCH_FORGET, &body(count, entries)),
            None,
            latest(),
        )
    }

    #[test]
    fn every_entry_carries_its_inode_and_count() {
        let req = decode(2, &[(5, 1), (6, 42)]).expect("decodes");
        let inos = req.inos();

        assert_eq!(inos.len(), 2);
        assert_eq!(inos[0].ino().as_raw(), 5);
        assert_eq!(inos[0].num_lookups(), 1);
        assert_eq!(inos[1].ino().as_raw(), 6);
        assert_eq!(inos[1].num_lookups(), 42);
    }

    #[test]
    fn forgetting_nothing_is_an_empty_list() {
        assert!(decode(0, &[]).expect("decodes").inos().is_empty());
    }

    // Inode zero names nothing, and `ForgetIno::ino` relies on never seeing it.
    #[test]
    fn an_entry_for_inode_zero_is_refused() {
        assert_eq!(
            decode(2, &[(5, 1), (0, 1)]).map(|_| ()).unwrap_err(),
            Error::EINVAL
        );
    }

    // A count larger than the entries that arrived would read past the
    // request, so it is refused rather than trusted.
    #[test]
    fn a_count_longer_than_the_body_is_refused() {
        assert_eq!(decode(4, &[(5, 1)]).map(|_| ()).unwrap_err(), Error::EPROTO);
    }

    // A count whose byte length overflows must not wrap into something that
    // looks like it fits.
    #[test]
    fn an_absurd_count_is_refused_rather_than_wrapping() {
        assert_eq!(
            decode(u32::MAX, &[(5, 1)]).map(|_| ()).unwrap_err(),
            Error::EPROTO
        );
    }

    // Entries past the announced count are not part of the request.
    #[test]
    fn entries_past_the_count_are_ignored() {
        let req = decode(1, &[(5, 1), (6, 1)]).expect("decodes");

        assert_eq!(req.inos().len(), 1);
        assert_eq!(req.inos()[0].ino().as_raw(), 5);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = BatchForget::decode(request(Opcode::BATCH_FORGET, &[0u8; 4]), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
