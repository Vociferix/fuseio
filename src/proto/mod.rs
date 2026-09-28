use crate::types::ReplyInitFlags;

pub mod notify;
pub mod request;
pub mod response;
pub(crate) mod time;

pub const MIN_MSG_SIZE: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    pub minor_ver: u32,
    /// Flags negotiated in the INIT reply.
    pub flags: ReplyInitFlags,
}

#[cfg(test)]
impl Default for Cfg {
    fn default() -> Self {
        Self {
            minor_ver: u32::MAX,
            flags: ReplyInitFlags::empty(),
        }
    }
}
