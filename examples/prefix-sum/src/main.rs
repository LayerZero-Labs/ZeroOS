//! Parallel prefix sum demo.
//!
//! Demonstrates stage-based parallel prefix sum computation,
//! which can be parallelized across ZeroOS cooperative threads.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use prefix_sum::{prefix_sum_blocked, prefix_sum_sequential, verify_prefix_sum};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Array size
const ARRAY_SIZE: usize = 64;

/// Number of parallel blocks
const NUM_BLOCKS: usize = 4;

/// Generate deterministic test data
fn generate_test_data(arr: &mut [u64]) {
    for i in 0..arr.len() {
        // Simple pattern: 1, 2, 3, ...
        arr[i] = (i + 1) as u64;
    }
}

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[prefix-sum] Starting prefix sum demo");
    debug::writeln!("[prefix-sum] Array size: {}, Blocks: {}", ARRAY_SIZE, NUM_BLOCKS);

    // Allocate arrays
    let mut arr = [0u64; ARRAY_SIZE];
    let mut out_seq = [0u64; ARRAY_SIZE];
    let mut out_blk = [0u64; ARRAY_SIZE];

    // Generate test data
    generate_test_data(&mut arr);

    println!("Input: [{}, {}, {}, {}, ...]", arr[0], arr[1], arr[2], arr[3]);

    // Compute sequential prefix sum
    debug::writeln!("[prefix-sum] Computing sequential prefix sum...");
    prefix_sum_sequential(&arr, &mut out_seq);

    // Compute blocked prefix sum
    debug::writeln!("[prefix-sum] Computing blocked prefix sum ({} blocks)...", NUM_BLOCKS);
    prefix_sum_blocked(&arr, &mut out_blk, NUM_BLOCKS);

    println!("Sequential: [{}, {}, {}, {}, ...]",
        out_seq[0], out_seq[1], out_seq[2], out_seq[3]);
    println!("Blocked:    [{}, {}, {}, {}, ...]",
        out_blk[0], out_blk[1], out_blk[2], out_blk[3]);

    // Verify sequential result
    let seq_valid = verify_prefix_sum(&arr, &out_seq);

    // Verify blocked matches sequential
    let mut blk_matches = true;
    for i in 0..ARRAY_SIZE {
        if out_seq[i] != out_blk[i] {
            blk_matches = false;
            break;
        }
    }

    if seq_valid && blk_matches {
        println!("Verification: PASSED");
        debug::writeln!("[prefix-sum] Verification PASSED");
    } else {
        println!("Verification: FAILED (seq_valid={}, blk_matches={})",
            seq_valid, blk_matches);
        debug::writeln!("[prefix-sum] Verification FAILED");
    }

    // Print final sum (should be n*(n+1)/2 for input 1,2,3,...,n)
    let expected_sum = (ARRAY_SIZE * (ARRAY_SIZE + 1) / 2) as u64;
    let actual_sum = out_seq[ARRAY_SIZE - 1];
    println!("Final sum: {} (expected: {})", actual_sum, expected_sum);

    debug::writeln!("[prefix-sum] Demo complete!");
    platform::exit(0)
}
