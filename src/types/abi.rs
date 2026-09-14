#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Abi {
    #[default]
    Native,
    Abi32BitOn64Bit,
    Abi32Bit,
    AbiX32,
}
