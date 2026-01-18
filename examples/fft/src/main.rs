//! FFT Example - Parallel Fast Fourier Transform
//!
//! Demonstrates stage-partitioned Cooley-Tukey FFT.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use fft::{batch_fft, Complex, TwiddleTable};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    println!("=== FFT Example ===");

    // Create twiddle factor table for 16-point FFT
    let twiddles = TwiddleTable::<16>::new();

    // Test 1: Simple DC signal (all ones)
    println!("\nTest 1: DC Signal [1,1,1,...,1]");
    let mut dc_signal: [Complex; 16] = [Complex::from_int(1); 16];

    fft::fft(&mut dc_signal, &twiddles.factors);

    println!("  Result[0] (DC): re={}", dc_signal[0].re >> 16);
    println!("  Expected: 16 (sum of inputs)");

    // Test 2: Impulse signal
    println!("\nTest 2: Impulse Signal [1,0,0,...,0]");
    let mut impulse: [Complex; 16] = [Complex::from_int(0); 16];
    impulse[0] = Complex::from_int(1);

    fft::fft(&mut impulse, &twiddles.factors);

    println!("  All bins should be ~1 (flat spectrum)");
    let mut all_ones = true;
    for (i, c) in impulse.iter().enumerate() {
        let mag = (c.re >> 16).abs();
        if mag < 1 {
            all_ones = false;
        }
        if i < 4 {
            println!("  Result[{}]: re={}", i, c.re >> 16);
        }
    }
    println!("  Flat spectrum: {}", if all_ones { "PASS" } else { "FAIL" });

    // Test 3: Simple sinusoid (alternating pattern = Nyquist)
    println!("\nTest 3: Nyquist Signal [1,-1,1,-1,...]");
    let mut nyquist: [Complex; 16] = [Complex::from_int(0); 16];
    for i in 0..16 {
        nyquist[i] = if i % 2 == 0 {
            Complex::from_int(1)
        } else {
            Complex::from_int(-1)
        };
    }

    fft::fft(&mut nyquist, &twiddles.factors);

    println!("  Result[0] (DC): {}", nyquist[0].re >> 16);
    println!("  Result[8] (Nyquist): {}", nyquist[8].re >> 16);
    println!("  Expected: DC=0, Nyquist=16");

    // Test 4: Batch FFT (parallel-friendly)
    println!("\nTest 4: Batch FFT (4 independent transforms)");

    let twiddles_8 = TwiddleTable::<8>::new();
    let mut batches: [[Complex; 8]; 4] = [[Complex::from_int(0); 8]; 4];

    // Initialize each batch differently
    for (batch_idx, batch) in batches.iter_mut().enumerate() {
        for (i, c) in batch.iter_mut().enumerate() {
            *c = Complex::from_int(((batch_idx + 1) * (i + 1)) as i32);
        }
    }

    batch_fft(&mut batches, &twiddles_8);

    for (batch_idx, batch) in batches.iter().enumerate() {
        println!(
            "  Batch {}: DC component = {}",
            batch_idx,
            batch[0].re >> 16
        );
    }

    // Test 5: Round-trip FFT -> IFFT
    println!("\nTest 5: FFT -> IFFT Round Trip");
    let original = [
        Complex::from_int(1),
        Complex::from_int(2),
        Complex::from_int(3),
        Complex::from_int(4),
        Complex::from_int(5),
        Complex::from_int(6),
        Complex::from_int(7),
        Complex::from_int(8),
    ];

    let mut data = original;
    let twiddles_8 = TwiddleTable::<8>::new();

    fft::fft(&mut data, &twiddles_8.factors);
    println!("  After FFT, DC = {}", data[0].re >> 16);

    fft::ifft(&mut data, &twiddles_8.factors);

    println!("  After IFFT:");
    let mut round_trip_ok = true;
    for i in 0..8 {
        let expected = (i + 1) as i32;
        let actual = data[i].re >> 16;
        if (actual - expected).abs() > 1 {
            round_trip_ok = false;
        }
        if i < 4 {
            println!("    [{}]: expected={}, actual={}", i, expected, actual);
        }
    }
    println!(
        "  Round-trip: {}",
        if round_trip_ok { "PASS" } else { "FAIL" }
    );

    println!("\n=== FFT Example Complete ===");

    platform::exit(0)
}
