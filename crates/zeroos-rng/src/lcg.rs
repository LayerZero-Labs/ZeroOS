use spin::Mutex;

#[repr(C)]
pub struct LcgState {
    state: u64,
}

impl Default for LcgState {
    fn default() -> Self {
        Self::new()
    }
}

impl LcgState {
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: 0xDEADBEEF_CAFEBABE,
        }
    }

    #[inline]
    pub const fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        let mut offset = 0;
        while offset < buf.len() {
            let rand = self.next_u64();
            let bytes = rand.to_le_bytes();
            let remaining = buf.len() - offset;
            let to_copy = remaining.min(8);
            buf[offset..offset + to_copy].copy_from_slice(&bytes[..to_copy]);
            offset += to_copy;
        }
    }
}

static GLOBAL_RNG: Mutex<LcgState> = Mutex::new(LcgState::new());

/// # Safety
/// - `buf` must be non-null and valid for writes of `len` bytes.
/// - `buf` must not alias any other active mutable reference for the duration of this call.
pub unsafe fn fill_bytes(buf: *mut u8, len: usize) -> isize {
    if buf.is_null() {
        return -1;
    }

    let mut rng = GLOBAL_RNG.lock();
    let slice = core::slice::from_raw_parts_mut(buf, len);
    rng.fill_bytes(slice);

    len as isize
}

pub fn init(seed: u64) {
    let mut rng = GLOBAL_RNG.lock();
    *rng = LcgState::with_seed(seed);
}
