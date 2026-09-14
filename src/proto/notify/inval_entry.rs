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
