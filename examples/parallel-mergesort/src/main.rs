//! Parallel merge sort demo.
//!
//! Demonstrates divide-and-conquer sorting using independent segment sorting,
//! which can be parallelized across ZeroOS cooperative threads.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use parallel_mergesort::{is_sorted, merge_segments, sort_segments};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Array size (keep small for ~1M cycle budget)
const ARRAY_SIZE: usize = 64;

/// Number of parallel segments
const NUM_SEGMENTS: usize = 4;

/// Generate deterministic test data
fn generate_test_data(arr: &mut [u32]) {
    let mut seed: u32 = 0x12345678;
    for i in 0..arr.len() {
        // Simple LCG for deterministic pseudo-random values
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        arr[i] = (seed >> 16) & 0xFFFF;
    }
}

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[parallel-mergesort] Starting merge sort demo");
    debug::writeln!("[parallel-mergesort] Array size: {}, Segments: {}", ARRAY_SIZE, NUM_SEGMENTS);

    // Allocate arrays
    let mut arr = [0u32; ARRAY_SIZE];
    let mut aux = [0u32; ARRAY_SIZE];

    // Generate test data
    generate_test_data(&mut arr);

    // Print first few elements before sorting
    println!("Before: [{}, {}, {}, {}, ...]", arr[0], arr[1], arr[2], arr[3]);

    // Phase 1: Sort independent segments (parallelizable)
    debug::writeln!("[parallel-mergesort] Sorting {} segments...", NUM_SEGMENTS);
    sort_segments(&mut arr, &mut aux, NUM_SEGMENTS);

    // Phase 2: Merge sorted segments
    debug::writeln!("[parallel-mergesort] Merging segments...");
    merge_segments(&mut arr, &mut aux, NUM_SEGMENTS);

    // Print first few elements after sorting
    println!("After:  [{}, {}, {}, {}, ...]", arr[0], arr[1], arr[2], arr[3]);

    // Verify sorted
    if is_sorted(&arr) {
        println!("Sort verification: PASSED");
        debug::writeln!("[parallel-mergesort] Sort verification PASSED");
    } else {
        println!("Sort verification: FAILED");
        debug::writeln!("[parallel-mergesort] Sort verification FAILED");
    }

    debug::writeln!("[parallel-mergesort] Demo complete!");
    platform::exit(0)
}
