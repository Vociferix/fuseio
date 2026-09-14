#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum XattrMode {
    #[default]
    CreateOrReplace,
    CreateOnly,
    ReplaceOnly,
}

impl XattrMode {
    pub fn create_only(&self) -> bool {
        matches!(self, Self::CreateOnly)
    }

    pub fn replace_only(&self) -> bool {
        matches!(self, Self::ReplaceOnly)
    }
}
