//! Parallel FFT Implementation (Cooley-Tukey)
//!
//! Stage-partitioned FFT for parallel execution.
//! Each stage operates on independent butterfly pairs.

#![no_std]

/// Complex number representation using fixed-point arithmetic.
/// Uses Q16.16 format for deterministic computation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub re: i32, // Q16.16 fixed-point
    pub im: i32, // Q16.16 fixed-point
}

impl Complex {
    pub const SCALE: i32 = 1 << 16;

    pub const fn new(re: i32, im: i32) -> Self {
        Self { re, im }
    }

    pub const fn from_int(re: i32) -> Self {
        Self {
            re: re << 16,
            im: 0,
        }
    }

    /// Fixed-point multiplication
    pub fn mul(self, other: Self) -> Self {
        let re = ((self.re as i64 * other.re as i64) >> 16)
            - ((self.im as i64 * other.im as i64) >> 16);
        let im = ((self.re as i64 * other.im as i64) >> 16)
            + ((self.im as i64 * other.re as i64) >> 16);
        Self {
            re: re as i32,
            im: im as i32,
        }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    /// Approximate magnitude squared (for validation)
    pub fn mag_squared(self) -> i64 {
        (self.re as i64 * self.re as i64 + self.im as i64 * self.im as i64) >> 16
    }
}

/// Pre-computed twiddle factors for FFT stages.
/// For N-point FFT: W_N^k = exp(-2πik/N)
pub struct TwiddleTable<const N: usize> {
    pub factors: [Complex; N],
}

impl<const N: usize> TwiddleTable<N> {
    /// Create twiddle factor table using integer approximation.
    /// Uses pre-computed sine table for determinism.
    pub fn new() -> Self {
        let mut factors = [Complex::new(0, 0); N];

        // Pre-computed sin/cos values for common angles
        // sin(2πk/N) approximated using Taylor series or lookup
        for k in 0..N {
            // For small N, use lookup table approach
            // cos(2πk/N) and -sin(2πk/N) in Q16.16
            let (cos_val, sin_val) = trig_lookup(k, N);
            factors[k] = Complex::new(cos_val, -sin_val);
        }

        Self { factors }
    }

    pub fn get(&self, k: usize) -> Complex {
        self.factors[k % N]
    }
}

impl<const N: usize> Default for TwiddleTable<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Integer trigonometry lookup using Cordic-style approximation.
/// Returns (cos, sin) in Q16.16 format.
fn trig_lookup(k: usize, n: usize) -> (i32, i32) {
    if n == 0 {
        return (Complex::SCALE, 0);
    }

    // Normalize angle to [0, 4) representing quadrants
    let angle_frac = (k * 4) / n;
    let sub_angle = (k * 4) % n;

    // Base sin/cos for quarter rotation
    // Using simple linear interpolation for small FFTs
    let t = (sub_angle as i64 * Complex::SCALE as i64) / n as i64;
    let t = t as i32;

    // Approximate cos and sin for angle in [0, π/2]
    // cos(x) ≈ 1 - x²/2, sin(x) ≈ x (for small x)
    // For better accuracy, use quadrant-aware computation
    let (base_cos, base_sin) = match angle_frac % 4 {
        0 => {
            // First quadrant: angle = t * π/2
            let c = Complex::SCALE - ((t as i64 * t as i64) >> 18) as i32;
            let s = (t as i64 * 102944 >> 16) as i32; // π/2 * scale ≈ 102944
            (c, s)
        }
        1 => {
            // Second quadrant
            let c = -((t as i64 * 102944 >> 16) as i32);
            let s = Complex::SCALE - ((t as i64 * t as i64) >> 18) as i32;
            (c, s)
        }
        2 => {
            // Third quadrant
            let c = -(Complex::SCALE - ((t as i64 * t as i64) >> 18) as i32);
            let s = -((t as i64 * 102944 >> 16) as i32);
            (c, s)
        }
        3 => {
            // Fourth quadrant
            let c = (t as i64 * 102944 >> 16) as i32;
            let s = -(Complex::SCALE - ((t as i64 * t as i64) >> 18) as i32);
            (c, s)
        }
        _ => unreachable!(),
    };

    (base_cos, base_sin)
}

/// Bit-reverse permutation index for FFT input reordering.
pub fn bit_reverse(mut x: usize, bits: u32) -> usize {
    let mut result = 0;
    for _ in 0..bits {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    result
}

/// In-place bit-reversal permutation of the input array.
pub fn bit_reverse_permute(data: &mut [Complex]) {
    let n = data.len();
    // Use integer log2 via trailing_zeros (n is power of 2)
    let bits = n.trailing_zeros();

    for i in 0..n {
        let j = bit_reverse(i, bits);
        if i < j {
            data.swap(i, j);
        }
    }
}

/// Single butterfly operation: the core FFT building block.
#[inline]
pub fn butterfly(a: &mut Complex, b: &mut Complex, twiddle: Complex) {
    let t = twiddle.mul(*b);
    let new_a = a.add(t);
    let new_b = a.sub(t);
    *a = new_a;
    *b = new_b;
}

/// FFT stage computation.
/// Each stage processes butterflies with a specific stride.
/// This is the parallelizable unit - different groups within a stage are independent.
pub fn fft_stage(data: &mut [Complex], stage: u32, twiddles: &[Complex]) {
    let n = data.len();
    let butterflies_per_group = 1 << stage;
    let group_size = butterflies_per_group * 2;
    let num_groups = n / group_size;

    // Each group can be processed independently (parallel-friendly)
    for group in 0..num_groups {
        let group_start = group * group_size;

        for k in 0..butterflies_per_group {
            let i = group_start + k;
            let j = i + butterflies_per_group;

            // Twiddle factor index: k * (N / group_size)
            let twiddle_idx = k * (n / group_size);
            let twiddle = twiddles[twiddle_idx % twiddles.len()];

            // Use split_at_mut to get non-overlapping mutable references
            let (left, right) = data.split_at_mut(j);
            butterfly(&mut left[i], &mut right[0], twiddle);
        }
    }
}

/// Complete FFT computation using Cooley-Tukey decimation-in-time.
/// Parallel-friendly: each stage can be partitioned across workers.
pub fn fft(data: &mut [Complex], twiddles: &[Complex]) {
    let n = data.len();
    assert!(n.is_power_of_two(), "FFT size must be power of 2");

    // Use integer log2 via trailing_zeros (n is power of 2)
    let num_stages = n.trailing_zeros();

    // Step 1: Bit-reverse permutation
    bit_reverse_permute(data);

    // Step 2: Process each stage
    // TODO: With threading, parallelize within each stage
    for stage in 0..num_stages {
        fft_stage(data, stage, twiddles);
    }
}

/// Inverse FFT (IFFT) - conjugate input, FFT, conjugate output, scale.
pub fn ifft(data: &mut [Complex], twiddles: &[Complex]) {
    let n = data.len();

    // Conjugate input
    for x in data.iter_mut() {
        x.im = -x.im;
    }

    // Forward FFT
    fft(data, twiddles);

    // Conjugate and scale output
    for x in data.iter_mut() {
        x.im = -x.im;
        x.re /= n as i32;
        x.im /= n as i32;
    }
}

/// Batch FFT processing - multiple independent FFTs.
/// Each FFT is completely independent (embarrassingly parallel).
pub fn batch_fft<const N: usize>(
    batches: &mut [[Complex; N]],
    twiddles: &TwiddleTable<N>,
) {
    // Each batch can be processed by a different thread
    for batch in batches.iter_mut() {
        fft(batch, &twiddles.factors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_arithmetic() {
        let a = Complex::from_int(3);
        let b = Complex::from_int(4);

        let sum = a.add(b);
        assert_eq!(sum.re, 7 << 16);
        assert_eq!(sum.im, 0);

        let diff = a.sub(b);
        assert_eq!(diff.re, -1 << 16);
    }

    #[test]
    fn test_bit_reverse() {
        assert_eq!(bit_reverse(0b000, 3), 0b000);
        assert_eq!(bit_reverse(0b001, 3), 0b100);
        assert_eq!(bit_reverse(0b010, 3), 0b010);
        assert_eq!(bit_reverse(0b011, 3), 0b110);
        assert_eq!(bit_reverse(0b100, 3), 0b001);
    }

    #[test]
    fn test_small_fft() {
        // 4-point FFT of [1, 1, 1, 1] should give [4, 0, 0, 0]
        let mut data = [
            Complex::from_int(1),
            Complex::from_int(1),
            Complex::from_int(1),
            Complex::from_int(1),
        ];

        let twiddles = TwiddleTable::<4>::new();
        fft(&mut data, &twiddles.factors);

        // First element should be sum = 4
        assert!((data[0].re - (4 << 16)).abs() < 1000);
        // Other elements should be near zero
        assert!(data[1].mag_squared() < 10000);
        assert!(data[2].mag_squared() < 10000);
        assert!(data[3].mag_squared() < 10000);
    }

    #[test]
    fn test_impulse_response() {
        // FFT of impulse [1, 0, 0, 0] should give all ones
        let mut data = [
            Complex::from_int(1),
            Complex::from_int(0),
            Complex::from_int(0),
            Complex::from_int(0),
        ];

        let twiddles = TwiddleTable::<4>::new();
        fft(&mut data, &twiddles.factors);

        // All elements should be approximately 1
        for c in &data {
            assert!((c.re - Complex::SCALE).abs() < 5000);
        }
    }
}
