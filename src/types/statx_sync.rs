#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum StatXSync {
    #[default]
    AsStat,
    DontSync,
    Force,
}
