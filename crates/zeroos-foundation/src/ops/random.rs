#[derive(Clone, Copy)]
pub struct RandomOps {
    pub init: fn(seed: u64),
    pub fill_bytes: unsafe fn(buf: *mut u8, len: usize) -> isize,
}
