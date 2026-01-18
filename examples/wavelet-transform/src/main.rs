//! Wavelet Transform Example
//!
//! Demonstrates level-partitioned Haar wavelet transform.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use wavelet_transform::{
    batch_transform, haar_2d_level, level_energy, threshold_details, HaarTransform,
};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    println!("=== Wavelet Transform Example ===");

    // Test 1: Simple 1D transform
    println!("\nTest 1: 1D Haar Transform");
    let input = [1, 3, 5, 7, 9, 11, 13, 15];
    let mut transform = HaarTransform::<8>::new();

    transform.transform(&input);

    println!("  Input: {:?}", input);
    println!("  Levels computed: {}", transform.num_levels);
    println!("  Final average: {}", transform.averages[0]);
    println!("  Level 0 details: [{}, {}, {}, {}]",
             transform.details[0][0], transform.details[0][1],
             transform.details[0][2], transform.details[0][3]);
    println!("  Level 1 details: [{}, {}]",
             transform.details[1][0], transform.details[1][1]);
    println!("  Level 2 details: [{}]", transform.details[2][0]);

    // Test 2: Round-trip reconstruction
    println!("\nTest 2: Reconstruction");
    let mut output = [0i32; 8];
    transform.inverse(&mut output, 8);

    println!("  Reconstructed: {:?}", output);
    let matches = output == input;
    println!("  Perfect reconstruction: {}", if matches { "PASS" } else { "FAIL" });

    // Test 3: Signal with edge
    println!("\nTest 3: Edge Detection");
    let edge_signal = [0, 0, 0, 0, 10, 10, 10, 10];
    let mut edge_transform = HaarTransform::<8>::new();

    edge_transform.transform(&edge_signal);

    println!("  Input (step edge): {:?}", edge_signal);
    println!("  Level 0 details: [{}, {}, {}, {}]",
             edge_transform.details[0][0], edge_transform.details[0][1],
             edge_transform.details[0][2], edge_transform.details[0][3]);
    println!("  Edge detected at level 1: {}", edge_transform.details[1][0] != 0);

    // Test 4: Energy computation
    println!("\nTest 4: Level Energy");
    let energy_0 = level_energy(&edge_transform.details[0], 4);
    let energy_1 = level_energy(&edge_transform.details[1], 2);
    let energy_2 = level_energy(&edge_transform.details[2], 1);

    println!("  Level 0 energy: {}", energy_0);
    println!("  Level 1 energy: {}", energy_1);
    println!("  Level 2 energy: {}", energy_2);

    // Test 5: Denoising via thresholding
    println!("\nTest 5: Denoising");
    let noisy = [10, 11, 9, 12, 50, 51, 49, 52];
    let mut noisy_transform = HaarTransform::<8>::new();
    noisy_transform.transform(&noisy);

    println!("  Noisy input: {:?}", noisy);
    println!("  Before threshold, level 0: [{}, {}, {}, {}]",
             noisy_transform.details[0][0], noisy_transform.details[0][1],
             noisy_transform.details[0][2], noisy_transform.details[0][3]);

    // Threshold small details (noise)
    threshold_details(&mut noisy_transform.details[0], 4, 2);

    println!("  After threshold (t=2): [{}, {}, {}, {}]",
             noisy_transform.details[0][0], noisy_transform.details[0][1],
             noisy_transform.details[0][2], noisy_transform.details[0][3]);

    let mut denoised = [0i32; 8];
    noisy_transform.inverse(&mut denoised, 8);
    println!("  Denoised output: {:?}", denoised);

    // Test 6: Batch transform (parallel-friendly)
    println!("\nTest 6: Batch Transform (4 signals)");
    let signals: [[i32; 8]; 4] = [
        [1, 2, 3, 4, 5, 6, 7, 8],
        [8, 7, 6, 5, 4, 3, 2, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [0, 1, 0, 1, 0, 1, 0, 1],
    ];

    let mut transforms = [
        HaarTransform::<8>::new(),
        HaarTransform::<8>::new(),
        HaarTransform::<8>::new(),
        HaarTransform::<8>::new(),
    ];

    batch_transform(&signals, &mut transforms);

    for (i, t) in transforms.iter().enumerate() {
        println!("  Signal {}: avg={}, level0_energy={}",
                 i, t.averages[0],
                 level_energy(&t.details[0], 4));
    }

    // Test 7: 2D transform (for images)
    println!("\nTest 7: 2D Haar Transform (8x8 block)");
    let image: [[i32; 8]; 8] = [
        [10, 10, 10, 10, 20, 20, 20, 20],
        [10, 10, 10, 10, 20, 20, 20, 20],
        [10, 10, 10, 10, 20, 20, 20, 20],
        [10, 10, 10, 10, 20, 20, 20, 20],
        [30, 30, 30, 30, 40, 40, 40, 40],
        [30, 30, 30, 30, 40, 40, 40, 40],
        [30, 30, 30, 30, 40, 40, 40, 40],
        [30, 30, 30, 30, 40, 40, 40, 40],
    ];

    let mut ll = [[0i32; 4]; 4];
    let mut lh = [[0i32; 4]; 4];
    let mut hl = [[0i32; 4]; 4];
    let mut hh = [[0i32; 4]; 4];

    haar_2d_level(&image, &mut ll, &mut lh, &mut hl, &mut hh);

    println!("  LL (approximation) corner: {}", ll[0][0]);
    println!("  LH (horizontal) corner: {}", lh[0][0]);
    println!("  HL (vertical) corner: {}", hl[0][0]);
    println!("  HH (diagonal) corner: {}", hh[0][0]);

    // Check that edges are detected
    let has_vertical_edge = hl.iter().any(|row| row.iter().any(|&x| x != 0));
    let has_horizontal_edge = lh.iter().any(|row| row.iter().any(|&x| x != 0));
    println!("  Vertical edge detected: {}", has_vertical_edge);
    println!("  Horizontal edge detected: {}", has_horizontal_edge);

    println!("\n=== Wavelet Transform Example Complete ===");

    platform::exit(0)
}
