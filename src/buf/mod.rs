mod buf;
mod pool;

pub use buf::Buf;
pub use pool::BufPool;

const ALIGN: usize = std::mem::align_of::<crossbeam_utils::CachePadded<u64>>();
