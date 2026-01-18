//! Keccak-f[1600] permutation implementation.
//!
//! This is a minimal, no_std-compatible implementation of the Keccak
//! permutation used in SHA3 and other sponge constructions.

#![no_std]

/// Keccak-f[1600] state: 5x5 array of 64-bit lanes (1600 bits total)
pub type KeccakState = [[u64; 5]; 5];

/// Round constants for Keccak-f[1600]
const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

/// Rotation offsets for rho step
const RHO: [[u32; 5]; 5] = [
    [0, 1, 62, 28, 27],
    [36, 44, 6, 55, 20],
    [3, 10, 43, 25, 39],
    [41, 45, 15, 21, 8],
    [18, 2, 61, 56, 14],
];

/// Perform the Keccak-f[1600] permutation on the state
pub fn keccak_f(state: &mut KeccakState) {
    for round in 0..24 {
        // θ (theta) step
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x][0] ^ state[x][1] ^ state[x][2] ^ state[x][3] ^ state[x][4];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        for x in 0..5 {
            for y in 0..5 {
                state[x][y] ^= d[x];
            }
        }

        // ρ (rho) and π (pi) steps combined
        let mut b = [[0u64; 5]; 5];
        for x in 0..5 {
            for y in 0..5 {
                b[y][(2 * x + 3 * y) % 5] = state[x][y].rotate_left(RHO[x][y]);
            }
        }

        // χ (chi) step
        for x in 0..5 {
            for y in 0..5 {
                state[x][y] = b[x][y] ^ ((!b[(x + 1) % 5][y]) & b[(x + 2) % 5][y]);
            }
        }

        // ι (iota) step
        state[0][0] ^= RC[round];
    }
}

/// Initialize state from a message block (simplified: just XOR first bytes)
pub fn absorb_block(state: &mut KeccakState, block: &[u8]) {
    // Rate for SHA3-256 is 1088 bits = 136 bytes = 17 lanes
    let lanes = core::cmp::min(block.len() / 8, 17);
    for i in 0..lanes {
        let x = i % 5;
        let y = i / 5;
        let mut lane_bytes = [0u8; 8];
        let start = i * 8;
        let end = core::cmp::min(start + 8, block.len());
        lane_bytes[..end - start].copy_from_slice(&block[start..end]);
        state[x][y] ^= u64::from_le_bytes(lane_bytes);
    }
}

/// Extract hash output from state (256 bits for SHA3-256)
pub fn squeeze_256(state: &KeccakState) -> [u8; 32] {
    let mut output = [0u8; 32];
    for i in 0..4 {
        let x = i % 5;
        let y = i / 5;
        let lane_bytes = state[x][y].to_le_bytes();
        output[i * 8..(i + 1) * 8].copy_from_slice(&lane_bytes);
    }
    output
}

/// Simple SHA3-256 hash of a single block (for demo purposes)
pub fn sha3_256_simple(data: &[u8]) -> [u8; 32] {
    let mut state: KeccakState = [[0u64; 5]; 5];

    // Absorb (simplified: single block, no proper padding)
    absorb_block(&mut state, data);

    // Apply permutation
    keccak_f(&mut state);

    // Squeeze
    squeeze_256(&state)
}

/// Batch hash multiple messages (single-threaded baseline)
pub fn batch_hash(messages: &[&[u8]], outputs: &mut [[u8; 32]]) {
    for (i, msg) in messages.iter().enumerate() {
        outputs[i] = sha3_256_simple(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak_f_deterministic() {
        let mut state1: KeccakState = [[0u64; 5]; 5];
        let mut state2: KeccakState = [[0u64; 5]; 5];
        state1[0][0] = 0x123456789ABCDEF0;
        state2[0][0] = 0x123456789ABCDEF0;

        keccak_f(&mut state1);
        keccak_f(&mut state2);

        assert_eq!(state1, state2);
    }

    #[test]
    fn test_sha3_simple() {
        let data = b"hello world";
        let hash1 = sha3_256_simple(data);
        let hash2 = sha3_256_simple(data);
        assert_eq!(hash1, hash2);
        // Hash should be non-zero
        assert!(hash1.iter().any(|&b| b != 0));
    }
}
