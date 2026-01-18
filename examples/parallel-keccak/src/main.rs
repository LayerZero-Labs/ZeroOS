//! Parallel Keccak/SHA3 batch hasher demo.
//!
//! Demonstrates multi-threaded hashing using ZeroOS cooperative threads.
//! Each worker thread processes a subset of message blocks independently.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use parallel_keccak::{sha3_256_simple, KeccakState, keccak_f};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Number of messages to hash in the batch
const BATCH_SIZE: usize = 8;

/// Test messages (deterministic for reproducibility)
const TEST_MESSAGES: [&[u8]; BATCH_SIZE] = [
    b"message_0_hello_world",
    b"message_1_foo_bar_baz",
    b"message_2_test_data_x",
    b"message_3_zkvm_rocks!",
    b"message_4_zeroos_demo",
    b"message_5_jolt_prover",
    b"message_6_keccak_hash",
    b"message_7_final_block",
];

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[parallel-keccak] Starting batch hash demo");

    // Single-threaded batch hash for now
    // TODO: Add multi-threaded version using ZeroOS threads
    let mut outputs = [[0u8; 32]; BATCH_SIZE];

    for (i, msg) in TEST_MESSAGES.iter().enumerate() {
        debug::writeln!("[parallel-keccak] Hashing message {}", i);
        outputs[i] = sha3_256_simple(msg);
    }

    // Print results (first 8 bytes of each hash)
    for (i, hash) in outputs.iter().enumerate() {
        println!(
            "hash[{}] = {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}...",
            i,
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7]
        );
    }

    // Verify determinism: hash the same message twice
    let check1 = sha3_256_simple(b"determinism_check");
    let check2 = sha3_256_simple(b"determinism_check");
    if check1 == check2 {
        debug::writeln!("[parallel-keccak] Determinism check PASSED");
    } else {
        debug::writeln!("[parallel-keccak] Determinism check FAILED");
    }

    debug::writeln!("[parallel-keccak] Demo complete!");
    platform::exit(0)
}
