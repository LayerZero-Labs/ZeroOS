//! Parallel Polynomial Evaluation
//!
//! Batch polynomial evaluation for parallel execution.
//! Each polynomial/point evaluation is independent.
//!
//! Useful for:
//! - Polynomial commitment schemes
//! - Reed-Solomon encoding
//! - Lagrange interpolation evaluation
//! - KZG proofs

#![no_std]

/// Maximum polynomial degree supported
pub const MAX_DEGREE: usize = 64;
/// Maximum number of evaluation points
pub const MAX_POINTS: usize = 64;
/// Prime field modulus (small prime for demo)
pub const MODULUS: i64 = 0x7FFFFFFF; // 2^31 - 1 (Mersenne prime)

/// Polynomial represented by coefficients.
/// p(x) = coeffs[0] + coeffs[1]*x + coeffs[2]*x^2 + ...
#[derive(Clone)]
pub struct Polynomial {
    pub coeffs: [i64; MAX_DEGREE],
    pub degree: usize,
}

impl Polynomial {
    pub const fn new() -> Self {
        Self {
            coeffs: [0; MAX_DEGREE],
            degree: 0,
        }
    }

    /// Create polynomial from coefficient slice.
    pub fn from_coeffs(coeffs: &[i64]) -> Self {
        let mut poly = Self::new();
        let len = core::cmp::min(coeffs.len(), MAX_DEGREE);
        for (i, &c) in coeffs.iter().take(len).enumerate() {
            poly.coeffs[i] = c % MODULUS;
        }
        poly.degree = len.saturating_sub(1);
        poly
    }

    /// Evaluate polynomial at point x using Horner's method.
    /// O(n) where n is degree.
    pub fn eval(&self, x: i64) -> i64 {
        let mut result = 0i64;

        // Horner's method: p(x) = c[0] + x*(c[1] + x*(c[2] + ...))
        for i in (0..=self.degree).rev() {
            result = (result.wrapping_mul(x) + self.coeffs[i]) % MODULUS;
            if result < 0 {
                result += MODULUS;
            }
        }

        result
    }

    /// Evaluate at multiple points (parallel-friendly).
    /// Each point evaluation is independent.
    pub fn eval_many(&self, points: &[i64], results: &mut [i64]) {
        assert!(points.len() <= results.len());

        // Each evaluation can be done by a different thread
        for (i, &x) in points.iter().enumerate() {
            results[i] = self.eval(x);
        }
    }

    /// Add two polynomials.
    pub fn add(&self, other: &Self) -> Self {
        let mut result = Self::new();
        let max_deg = core::cmp::max(self.degree, other.degree);

        for i in 0..=max_deg {
            let a = if i <= self.degree { self.coeffs[i] } else { 0 };
            let b = if i <= other.degree { other.coeffs[i] } else { 0 };
            result.coeffs[i] = (a + b) % MODULUS;
        }
        result.degree = max_deg;

        result
    }

    /// Multiply polynomial by scalar.
    pub fn scale(&self, scalar: i64) -> Self {
        let mut result = Self::new();
        result.degree = self.degree;

        for i in 0..=self.degree {
            result.coeffs[i] = (self.coeffs[i].wrapping_mul(scalar)) % MODULUS;
        }

        result
    }
}

impl Default for Polynomial {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch polynomial evaluation.
/// Evaluate multiple polynomials at multiple points.
/// Each (poly, point) pair is independent (embarrassingly parallel).
pub fn batch_eval(
    polys: &[Polynomial],
    points: &[i64],
    results: &mut [[i64; MAX_POINTS]],
) {
    assert!(polys.len() <= results.len());
    assert!(points.len() <= MAX_POINTS);

    // Each polynomial can be evaluated by a different thread
    for (i, poly) in polys.iter().enumerate() {
        // Each point within a polynomial can also be parallelized
        for (j, &x) in points.iter().enumerate() {
            results[i][j] = poly.eval(x);
        }
    }
}

/// Lagrange basis polynomial L_i(x) at evaluation point.
/// L_i(x) = ∏_{j≠i} (x - x_j) / (x_i - x_j)
pub fn lagrange_basis(points: &[i64], i: usize, x: i64) -> i64 {
    let mut numerator: i64 = 1;
    let mut denominator: i64 = 1;

    for (j, &x_j) in points.iter().enumerate() {
        if j != i {
            numerator = (numerator.wrapping_mul(x - x_j)) % MODULUS;
            denominator = (denominator.wrapping_mul(points[i] - x_j)) % MODULUS;
        }
    }

    // Modular division: numerator * denominator^(-1)
    // Using Fermat's little theorem: a^(-1) = a^(p-2) mod p
    let inv = mod_pow(denominator, MODULUS - 2);
    (numerator.wrapping_mul(inv)) % MODULUS
}

/// Modular exponentiation using binary method.
pub fn mod_pow(base: i64, mut exp: i64) -> i64 {
    let mut result = 1i64;
    let mut base = base % MODULUS;

    while exp > 0 {
        if exp & 1 == 1 {
            result = (result.wrapping_mul(base)) % MODULUS;
        }
        exp >>= 1;
        base = (base.wrapping_mul(base)) % MODULUS;
    }

    if result < 0 {
        result + MODULUS
    } else {
        result
    }
}

/// Lagrange interpolation at point x.
/// Given (x_i, y_i) pairs, compute p(x) where p interpolates all points.
pub fn lagrange_interpolate(xs: &[i64], ys: &[i64], x: i64) -> i64 {
    assert_eq!(xs.len(), ys.len());

    let mut result = 0i64;

    // Each term can be computed independently (parallel-friendly)
    for (i, &y_i) in ys.iter().enumerate() {
        let basis = lagrange_basis(xs, i, x);
        result = (result + y_i.wrapping_mul(basis)) % MODULUS;
    }

    if result < 0 {
        result + MODULUS
    } else {
        result
    }
}

/// Multi-point Lagrange interpolation.
/// Interpolate at multiple evaluation points (parallel-friendly).
pub fn lagrange_interpolate_many(
    xs: &[i64],
    ys: &[i64],
    eval_points: &[i64],
    results: &mut [i64],
) {
    assert_eq!(xs.len(), ys.len());
    assert!(eval_points.len() <= results.len());

    // Each evaluation point is independent
    for (i, &x) in eval_points.iter().enumerate() {
        results[i] = lagrange_interpolate(xs, ys, x);
    }
}

/// Polynomial multiplication (convolution).
/// Useful for combining polynomials.
pub fn poly_mul(a: &Polynomial, b: &Polynomial) -> Polynomial {
    let mut result = Polynomial::new();
    let result_degree = a.degree + b.degree;

    if result_degree >= MAX_DEGREE {
        // Overflow protection
        result.degree = MAX_DEGREE - 1;
    } else {
        result.degree = result_degree;
    }

    // Standard convolution - can be parallelized
    for i in 0..=a.degree {
        for j in 0..=b.degree {
            if i + j < MAX_DEGREE {
                let product = (a.coeffs[i].wrapping_mul(b.coeffs[j])) % MODULUS;
                result.coeffs[i + j] = (result.coeffs[i + j] + product) % MODULUS;
            }
        }
    }

    result
}

/// Reed-Solomon encoding: evaluate polynomial at consecutive powers of generator.
/// p(g^0), p(g^1), p(g^2), ..., p(g^(n-1))
pub fn rs_encode(poly: &Polynomial, generator: i64, num_points: usize, output: &mut [i64]) {
    assert!(num_points <= output.len());

    let mut g_power = 1i64;

    // Each evaluation is independent (parallel-friendly)
    for result in output.iter_mut().take(num_points) {
        *result = poly.eval(g_power);
        g_power = (g_power.wrapping_mul(generator)) % MODULUS;
    }
}

/// Compute polynomial derivative.
/// If p(x) = sum(c_i * x^i), then p'(x) = sum(i * c_i * x^(i-1))
pub fn derivative(poly: &Polynomial) -> Polynomial {
    let mut result = Polynomial::new();

    if poly.degree == 0 {
        return result;
    }

    result.degree = poly.degree - 1;
    for i in 1..=poly.degree {
        result.coeffs[i - 1] = (poly.coeffs[i].wrapping_mul(i as i64)) % MODULUS;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_poly() {
        let poly = Polynomial::from_coeffs(&[5]);
        assert_eq!(poly.eval(0), 5);
        assert_eq!(poly.eval(100), 5);
    }

    #[test]
    fn test_linear_poly() {
        // p(x) = 2 + 3x
        let poly = Polynomial::from_coeffs(&[2, 3]);
        assert_eq!(poly.eval(0), 2);
        assert_eq!(poly.eval(1), 5);
        assert_eq!(poly.eval(10), 32);
    }

    #[test]
    fn test_quadratic_poly() {
        // p(x) = 1 + 2x + x^2 = (1+x)^2
        let poly = Polynomial::from_coeffs(&[1, 2, 1]);
        assert_eq!(poly.eval(0), 1);
        assert_eq!(poly.eval(1), 4);
        assert_eq!(poly.eval(2), 9);
        assert_eq!(poly.eval(3), 16);
    }

    #[test]
    fn test_poly_add() {
        let a = Polynomial::from_coeffs(&[1, 2]); // 1 + 2x
        let b = Polynomial::from_coeffs(&[3, 4]); // 3 + 4x
        let c = a.add(&b); // 4 + 6x

        assert_eq!(c.eval(0), 4);
        assert_eq!(c.eval(1), 10);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(2, 10), 1024);
        assert_eq!(mod_pow(3, 0), 1);
        assert_eq!(mod_pow(5, 1), 5);
    }

    #[test]
    fn test_lagrange_simple() {
        // Interpolate through (0,1), (1,2), (2,5)
        // This is p(x) = 1 + 0.5x + 0.5x^2, but we use integer math
        let xs = [0, 1, 2];
        let ys = [1, 2, 5];

        // Verify interpolation at known points
        assert_eq!(lagrange_interpolate(&xs, &ys, 0), 1);
        assert_eq!(lagrange_interpolate(&xs, &ys, 1), 2);
        assert_eq!(lagrange_interpolate(&xs, &ys, 2), 5);
    }

    #[test]
    fn test_derivative() {
        // p(x) = 1 + 2x + 3x^2, p'(x) = 2 + 6x
        let poly = Polynomial::from_coeffs(&[1, 2, 3]);
        let deriv = derivative(&poly);

        assert_eq!(deriv.coeffs[0], 2);
        assert_eq!(deriv.coeffs[1], 6);
        assert_eq!(deriv.degree, 1);
    }

    #[test]
    fn test_batch_eval() {
        let polys = [
            Polynomial::from_coeffs(&[1, 1]), // 1 + x
            Polynomial::from_coeffs(&[0, 0, 1]), // x^2
        ];
        let points = [0, 1, 2, 3];
        let mut results = [[0i64; MAX_POINTS]; 2];

        batch_eval(&polys, &points, &mut results);

        // p1: 1+x at 0,1,2,3 = 1,2,3,4
        assert_eq!(results[0][0], 1);
        assert_eq!(results[0][1], 2);
        assert_eq!(results[0][2], 3);
        assert_eq!(results[0][3], 4);

        // p2: x^2 at 0,1,2,3 = 0,1,4,9
        assert_eq!(results[1][0], 0);
        assert_eq!(results[1][1], 1);
        assert_eq!(results[1][2], 4);
        assert_eq!(results[1][3], 9);
    }
}
