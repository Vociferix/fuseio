use super::entry::Entry;
use super::{Cfg, EncodeNotify, IntoIoBuf};
use crate::types::Ino;

#[derive(Debug)]
pub struct InvalEntry<B> {
    entry: Entry<B, false>,
}

impl<B: IntoIoBuf> InvalEntry<B> {
    pub fn new(parent: Ino, name: B) -> Self {
        Self {
            entry: Entry::new(parent, name),
        }
    }
}

impl<B: IntoIoBuf> EncodeNotify for InvalEntry<B> {
    type Error = <Entry<B, false> as EncodeNotify>::Error;

    fn encode(self, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        self.entry.encode(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode, u32_at, u64_at};

    const PARENT: usize = HDR;
    const NAMELEN: usize = HDR + 8;
    const FLAGS: usize = HDR + 12;
    const NAME: usize = HDR + 16;

    // `fuse_notify_inval_entry_out { parent, namelen, flags }`, code 3, then
    // the name. Both notifications share the code and differ only in the flag,
    // so picking the wrong one would compile and quietly do the other thing:
    // dropping a cached entry outright rather than letting it expire.
    #[test]
    fn an_invalidation_carries_the_name_and_no_flag() {
        let parent = Ino::from_raw(3).unwrap();
        let bytes = encode(InvalEntry::new(parent, b"child.txt".to_vec()));

        assert_notify_header(&bytes, 3);
        assert_eq!(u64_at(&bytes, PARENT), 3);
        assert_eq!(u32_at(&bytes, FLAGS), 0);
        assert_eq!(
            u32_at(&bytes, NAMELEN),
            9,
            "namelen counts the name without its terminator"
        );
        assert_eq!(
            &bytes[NAME..],
            b"child.txt\0",
            "the terminator belongs in the payload"
        );
        assert_eq!(bytes.len(), NAME + 10);
    }
}
