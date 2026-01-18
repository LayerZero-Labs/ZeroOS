//! Polynomial Evaluation Example
//!
//! Demonstrates parallel polynomial evaluation for zkVM applications.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use polynomial_eval::{
    batch_eval, derivative, lagrange_interpolate, lagrange_interpolate_many,
    mod_pow, poly_mul, rs_encode, Polynomial, MAX_POINTS, MODULUS,
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
    println!("=== Polynomial Evaluation Example ===");
    println!("  Using modulus: {}", MODULUS);

    // Test 1: Basic polynomial evaluation
    println!("\nTest 1: Basic Evaluation");
    // p(x) = 5 + 3x + 2x^2
    let poly = Polynomial::from_coeffs(&[5, 3, 2]);
    println!("  Polynomial: 5 + 3x + 2x^2");
    println!("  p(0) = {}", poly.eval(0));
    println!("  p(1) = {}", poly.eval(1));
    println!("  p(2) = {}", poly.eval(2));
    println!("  p(10) = {}", poly.eval(10));

    // Verify: p(2) = 5 + 6 + 8 = 19
    let expected = 5 + 3 * 2 + 2 * 4;
    println!("  Expected p(2) = {}: {}", expected, if poly.eval(2) == expected { "PASS" } else { "FAIL" });

    // Test 2: Multi-point evaluation
    println!("\nTest 2: Multi-Point Evaluation");
    let points = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut results = [0i64; 8];
    poly.eval_many(&points, &mut results);

    println!("  Points: 0..7");
    println!("  Results: {} {} {} {} {} {} {} {}",
             results[0], results[1], results[2], results[3],
             results[4], results[5], results[6], results[7]);

    // Test 3: Polynomial arithmetic
    println!("\nTest 3: Polynomial Arithmetic");
    let p1 = Polynomial::from_coeffs(&[1, 2]); // 1 + 2x
    let p2 = Polynomial::from_coeffs(&[3, 4]); // 3 + 4x

    let sum = p1.add(&p2); // 4 + 6x
    let product = poly_mul(&p1, &p2); // 3 + 10x + 8x^2

    println!("  p1 = 1 + 2x, p2 = 3 + 4x");
    println!("  p1 + p2 at x=1: {} (expected 10)", sum.eval(1));
    println!("  p1 * p2 at x=1: {} (expected 21)", product.eval(1));

    // Test 4: Lagrange interpolation
    println!("\nTest 4: Lagrange Interpolation");
    // Interpolate through (0,1), (1,4), (2,9), (3,16) = x^2 + 2x + 1 = (x+1)^2
    let xs = [0, 1, 2, 3];
    let ys = [1, 4, 9, 16];

    println!("  Points: (0,1), (1,4), (2,9), (3,16)");

    // Verify at known points
    let mut all_match = true;
    for i in 0..4 {
        let interp = lagrange_interpolate(&xs, &ys, xs[i]);
        if interp != ys[i] {
            all_match = false;
        }
    }
    println!("  Interpolation at known points: {}", if all_match { "PASS" } else { "FAIL" });

    // Evaluate at new points
    let p5 = lagrange_interpolate(&xs, &ys, 5);
    let expected_p5 = 36; // (5+1)^2
    println!("  p(5) = {} (expected {}): {}",
             p5, expected_p5, if p5 == expected_p5 { "PASS" } else { "FAIL" });

    // Test 5: Multi-point interpolation
    println!("\nTest 5: Multi-Point Interpolation");
    let eval_points = [4, 5, 6, 7];
    let mut interp_results = [0i64; 4];
    lagrange_interpolate_many(&xs, &ys, &eval_points, &mut interp_results);

    println!("  Interpolated values:");
    for (i, &x) in eval_points.iter().enumerate() {
        let expected = (x + 1) * (x + 1);
        println!("    p({}) = {} (expected {})", x, interp_results[i], expected);
    }

    // Test 6: Batch polynomial evaluation
    println!("\nTest 6: Batch Evaluation (parallel-friendly)");
    let polys = [
        Polynomial::from_coeffs(&[1, 1]),    // 1 + x
        Polynomial::from_coeffs(&[0, 0, 1]), // x^2
        Polynomial::from_coeffs(&[1, 1, 1]), // 1 + x + x^2
    ];
    let batch_points = [0, 1, 2, 3];
    let mut batch_results = [[0i64; MAX_POINTS]; 4];

    batch_eval(&polys, &batch_points, &mut batch_results);

    println!("  3 polynomials evaluated at 4 points each:");
    for (i, poly_name) in ["1+x", "x^2", "1+x+x^2"].iter().enumerate() {
        println!("    {}: {} {} {} {}",
                 poly_name,
                 batch_results[i][0], batch_results[i][1],
                 batch_results[i][2], batch_results[i][3]);
    }

    // Test 7: Reed-Solomon encoding
    println!("\nTest 7: Reed-Solomon Encoding");
    let message_poly = Polynomial::from_coeffs(&[1, 2, 3, 4]); // Message as polynomial
    let generator = 3; // Primitive root
    let mut codeword = [0i64; 8];

    rs_encode(&message_poly, generator, 8, &mut codeword);

    println!("  Message polynomial: 1 + 2x + 3x^2 + 4x^3");
    println!("  Generator: {}", generator);
    println!("  Codeword (8 points): {} {} {} {} {} {} {} {}",
             codeword[0], codeword[1], codeword[2], codeword[3],
             codeword[4], codeword[5], codeword[6], codeword[7]);

    // Test 8: Polynomial derivative
    println!("\nTest 8: Polynomial Derivative");
    let p = Polynomial::from_coeffs(&[1, 2, 3, 4]); // 1 + 2x + 3x^2 + 4x^3
    let dp = derivative(&p); // 2 + 6x + 12x^2

    println!("  p(x) = 1 + 2x + 3x^2 + 4x^3");
    println!("  p'(x) = 2 + 6x + 12x^2");
    println!("  p'(0) = {} (expected 2)", dp.eval(0));
    println!("  p'(1) = {} (expected 20)", dp.eval(1));
    println!("  p'(2) = {} (expected 62)", dp.eval(2));

    // Test 9: Modular arithmetic
    println!("\nTest 9: Modular Arithmetic");
    println!("  2^10 mod {} = {}", MODULUS, mod_pow(2, 10));
    println!("  3^100 mod {} = {}", MODULUS, mod_pow(3, 100));

    // Test Fermat's little theorem: a^(p-1) = 1 mod p
    let fermat = mod_pow(7, MODULUS - 1);
    println!("  7^(p-1) mod p = {} (Fermat: should be 1)", fermat);

    // Test 10: Zero polynomial
    println!("\nTest 10: Edge Cases");
    let zero_poly = Polynomial::from_coeffs(&[0]);
    let const_poly = Polynomial::from_coeffs(&[42]);

    println!("  Zero polynomial at x=100: {}", zero_poly.eval(100));
    println!("  Constant 42 at x=100: {}", const_poly.eval(100));

    // High degree polynomial
    let mut high_coeffs = [0i64; 32];
    for i in 0..32 {
        high_coeffs[i] = (i + 1) as i64;
    }
    let high_poly = Polynomial::from_coeffs(&high_coeffs);
    println!("  Degree-31 polynomial at x=2: {}", high_poly.eval(2));

    println!("\n=== Polynomial Evaluation Example Complete ===");

    platform::exit(0)
}
