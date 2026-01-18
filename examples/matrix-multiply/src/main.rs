//! Parallel matrix multiplication demo.
//!
//! Demonstrates block-based matrix multiplication where each row block
//! can be computed independently by a different ZeroOS thread.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use matrix_multiply::{
    init_matrix, matmul, matmul_blocked, matrices_equal, matrix_checksum, zero_matrix, DIM,
};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Number of parallel blocks (each can be a separate thread)
const NUM_BLOCKS: usize = 4;

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[matrix-multiply] Starting matrix multiplication demo");
    debug::writeln!("[matrix-multiply] Matrix size: {}x{}, Blocks: {}", DIM, DIM, NUM_BLOCKS);

    // Initialize matrices
    let a = init_matrix(0x12345678);
    let b = init_matrix(0xDEADBEEF);

    println!("Matrix A[0][0..3]: [{}, {}, {}, {}]", a[0][0], a[0][1], a[0][2], a[0][3]);
    println!("Matrix B[0][0..3]: [{}, {}, {}, {}]", b[0][0], b[0][1], b[0][2], b[0][3]);

    // Compute using standard algorithm
    debug::writeln!("[matrix-multiply] Computing standard matmul...");
    let mut c_std = zero_matrix();
    matmul(&a, &b, &mut c_std);

    // Compute using blocked algorithm
    debug::writeln!("[matrix-multiply] Computing blocked matmul ({} blocks)...", NUM_BLOCKS);
    let mut c_blk = zero_matrix();
    matmul_blocked(&a, &b, &mut c_blk, NUM_BLOCKS);

    // Verify results match
    if matrices_equal(&c_std, &c_blk) {
        println!("Verification: PASSED (standard == blocked)");
        debug::writeln!("[matrix-multiply] Verification PASSED");
    } else {
        println!("Verification: FAILED (mismatch!)");
        debug::writeln!("[matrix-multiply] Verification FAILED");
    }

    // Print result sample and checksum
    println!("Result C[0][0..3]: [{}, {}, {}, {}]", c_std[0][0], c_std[0][1], c_std[0][2], c_std[0][3]);

    let checksum = matrix_checksum(&c_std);
    println!("Result checksum: {}", checksum);

    debug::writeln!("[matrix-multiply] Demo complete!");
    platform::exit(0)
}
