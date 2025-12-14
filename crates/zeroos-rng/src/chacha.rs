use spin::Mutex;

#[repr(C)]
pub struct ChaChaState {
    state: [u32; 16],
    counter: u64,
}

impl Default for ChaChaState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChaChaState {
    #[inline]
    pub const fn new() -> Self {
        Self::with_seed(0xDEADBEEF_CAFEBABE)
    }

    pub const fn with_seed(seed: u64) -> Self {
        let mut state = [0u32; 16];
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        state[4] = (seed & 0xFFFFFFFF) as u32;
        state[5] = (seed >> 32) as u32;
        state[6] = (seed & 0xFFFFFFFF) as u32;
        state[7] = (seed >> 32) as u32;
        state[8] = !state[4];
        state[9] = !state[5];
        state[10] = !state[6];
        state[11] = !state[7];

        state[12] = 0;
        state[13] = 0;
        state[14] = 0;
        state[15] = 0;

        Self { state, counter: 0 }
    }

    #[inline]
    fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(16);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(12);

        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(8);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(7);
    }

    fn generate_block(&mut self, output: &mut [u8; 64]) {
        let mut working_state = self.state;

        working_state[12] = (self.counter & 0xFFFFFFFF) as u32;
        working_state[13] = (self.counter >> 32) as u32;
        self.counter = self.counter.wrapping_add(1);

        for _ in 0..10 {
            Self::quarter_round(&mut working_state, 0, 4, 8, 12);
            Self::quarter_round(&mut working_state, 1, 5, 9, 13);
            Self::quarter_round(&mut working_state, 2, 6, 10, 14);
            Self::quarter_round(&mut working_state, 3, 7, 11, 15);

            Self::quarter_round(&mut working_state, 0, 5, 10, 15);
            Self::quarter_round(&mut working_state, 1, 6, 11, 12);
            Self::quarter_round(&mut working_state, 2, 7, 8, 13);
            Self::quarter_round(&mut working_state, 3, 4, 9, 14);
        }

        for (word, addend) in working_state.iter_mut().zip(self.state.iter()) {
            *word = (*word).wrapping_add(*addend);
        }

        for (i, &word) in working_state.iter().enumerate() {
            let bytes = word.to_le_bytes();
            output[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }
    }

    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        let mut offset = 0;
        let mut block = [0u8; 64];

        while offset < buf.len() {
            self.generate_block(&mut block);
            let remaining = buf.len() - offset;
            let to_copy = remaining.min(64);
            buf[offset..offset + to_copy].copy_from_slice(&block[..to_copy]);
            offset += to_copy;
        }
    }
}

static GLOBAL_RNG: Mutex<ChaChaState> = Mutex::new(ChaChaState::new());

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
    *rng = ChaChaState::with_seed(seed);
}
