//! Parallel Wavelet Transform Implementation
//!
//! Haar wavelet transform with level-based partitioning.
//! Each level's coefficient pairs can be computed independently.

#![no_std]

/// Haar wavelet coefficients at a single level.
/// Average and detail coefficients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HaarCoeff {
    pub average: i32,
    pub detail: i32,
}

impl HaarCoeff {
    pub fn new(average: i32, detail: i32) -> Self {
        Self { average, detail }
    }
}

/// Single Haar transform step on two adjacent values.
/// Returns (average, detail) = ((a+b)/2, (a-b)/2)
#[inline]
pub fn haar_step(a: i32, b: i32) -> HaarCoeff {
    // Use integer arithmetic to avoid floating point
    // Scale up to preserve precision, then scale down
    HaarCoeff {
        average: (a + b) / 2,
        detail: (a - b) / 2,
    }
}

/// Inverse Haar step: reconstruct original values from coefficients.
#[inline]
pub fn haar_inverse_step(coeff: HaarCoeff) -> (i32, i32) {
    let a = coeff.average + coeff.detail;
    let b = coeff.average - coeff.detail;
    (a, b)
}

/// Single level of Haar transform.
/// Processes pairs of values independently (parallel-friendly).
pub fn haar_level(input: &[i32], averages: &mut [i32], details: &mut [i32]) {
    assert_eq!(input.len(), averages.len() * 2);
    assert_eq!(averages.len(), details.len());

    // Each pair is independent - can be parallelized
    for i in 0..averages.len() {
        let coeff = haar_step(input[i * 2], input[i * 2 + 1]);
        averages[i] = coeff.average;
        details[i] = coeff.detail;
    }
}

/// Multi-level Haar wavelet transform.
/// Returns all detail coefficients at each level plus final average.
pub struct HaarTransform<const N: usize> {
    /// Detail coefficients at each level (level 0 = finest)
    pub details: [[i32; N]; 8], // Support up to 256-element input
    /// Final averages at coarsest level
    pub averages: [i32; N],
    /// Number of levels computed
    pub num_levels: usize,
}

impl<const N: usize> HaarTransform<N> {
    pub fn new() -> Self {
        Self {
            details: [[0; N]; 8],
            averages: [0; N],
            num_levels: 0,
        }
    }

    /// Compute full Haar transform.
    /// Each level halves the data size.
    pub fn transform(&mut self, input: &[i32]) {
        let n = input.len();
        assert!(n.is_power_of_two() && n <= N);

        // Copy input to working buffer
        for (i, &val) in input.iter().enumerate() {
            self.averages[i] = val;
        }

        let mut current_len = n;
        let mut level = 0;

        // Process each level until we reach single value
        while current_len > 1 {
            let half_len = current_len / 2;

            // Compute this level's coefficients
            // TODO: With threading, each pair in this level is independent
            for i in 0..half_len {
                let coeff = haar_step(self.averages[i * 2], self.averages[i * 2 + 1]);
                self.details[level][i] = coeff.detail;
                // Store averages for next level (in-place)
                self.averages[i] = coeff.average;
            }

            current_len = half_len;
            level += 1;
        }

        self.num_levels = level;
    }

    /// Reconstruct original signal from transform coefficients.
    pub fn inverse(&self, output: &mut [i32], original_len: usize) {
        assert!(original_len.is_power_of_two() && original_len <= N);

        // Start with final average(s)
        output[0] = self.averages[0];
        let mut current_len = 1;

        // Reconstruct each level from coarsest to finest
        for level in (0..self.num_levels).rev() {
            // Each reconstruction step doubles the data
            // TODO: With threading, each pair reconstruction is independent
            for i in (0..current_len).rev() {
                let coeff = HaarCoeff::new(output[i], self.details[level][i]);
                let (a, b) = haar_inverse_step(coeff);
                output[i * 2] = a;
                output[i * 2 + 1] = b;
            }
            current_len *= 2;
        }
    }
}

impl<const N: usize> Default for HaarTransform<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch transform multiple signals.
/// Each signal is completely independent (embarrassingly parallel).
pub fn batch_transform<const N: usize>(
    inputs: &[[i32; N]],
    transforms: &mut [HaarTransform<N>],
) {
    assert_eq!(inputs.len(), transforms.len());

    // Each transform is independent
    for (input, transform) in inputs.iter().zip(transforms.iter_mut()) {
        transform.transform(input);
    }
}

/// Compute energy at each decomposition level.
/// Useful for signal analysis.
pub fn level_energy(details: &[i32], len: usize) -> i64 {
    let mut energy: i64 = 0;
    for &d in details.iter().take(len) {
        energy += (d as i64) * (d as i64);
    }
    energy
}

/// Simple thresholding for denoising.
/// Zeros out detail coefficients below threshold.
pub fn threshold_details(details: &mut [i32], len: usize, threshold: i32) {
    for d in details.iter_mut().take(len) {
        if d.abs() < threshold {
            *d = 0;
        }
    }
}

/// 2D Haar transform for images (separable).
/// Applies 1D transform to rows, then columns.
pub fn haar_2d_level(
    input: &[[i32; 8]; 8],
    ll: &mut [[i32; 4]; 4], // Low-low (approximation)
    lh: &mut [[i32; 4]; 4], // Low-high (horizontal detail)
    hl: &mut [[i32; 4]; 4], // High-low (vertical detail)
    hh: &mut [[i32; 4]; 4], // High-high (diagonal detail)
) {
    // Temporary storage for row transform
    let mut row_avg = [[0i32; 4]; 8];
    let mut row_det = [[0i32; 4]; 8];

    // Step 1: Transform rows (can be parallelized)
    for i in 0..8 {
        for j in 0..4 {
            let coeff = haar_step(input[i][j * 2], input[i][j * 2 + 1]);
            row_avg[i][j] = coeff.average;
            row_det[i][j] = coeff.detail;
        }
    }

    // Step 2: Transform columns (can be parallelized)
    for j in 0..4 {
        for i in 0..4 {
            // Transform average columns -> LL, LH
            let coeff_avg = haar_step(row_avg[i * 2][j], row_avg[i * 2 + 1][j]);
            ll[i][j] = coeff_avg.average;
            lh[i][j] = coeff_avg.detail;

            // Transform detail columns -> HL, HH
            let coeff_det = haar_step(row_det[i * 2][j], row_det[i * 2 + 1][j]);
            hl[i][j] = coeff_det.average;
            hh[i][j] = coeff_det.detail;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haar_step() {
        let coeff = haar_step(10, 6);
        assert_eq!(coeff.average, 8);
        assert_eq!(coeff.detail, 2);

        let (a, b) = haar_inverse_step(coeff);
        assert_eq!(a, 10);
        assert_eq!(b, 6);
    }

    #[test]
    fn test_haar_level() {
        let input = [4, 2, 8, 6];
        let mut avg = [0; 2];
        let mut det = [0; 2];

        haar_level(&input, &mut avg, &mut det);

        assert_eq!(avg, [3, 7]);
        assert_eq!(det, [1, 1]);
    }

    #[test]
    fn test_full_transform_inverse() {
        let input = [1, 3, 5, 7, 9, 11, 13, 15];
        let mut transform = HaarTransform::<8>::new();

        transform.transform(&input);

        let mut output = [0i32; 8];
        transform.inverse(&mut output, 8);

        assert_eq!(output, input);
    }

    #[test]
    fn test_dc_signal() {
        // Constant signal should have zero details
        let input = [4, 4, 4, 4];
        let mut transform = HaarTransform::<4>::new();

        transform.transform(&input);

        assert_eq!(transform.averages[0], 4);
        assert_eq!(transform.details[0][0], 0);
        assert_eq!(transform.details[0][1], 0);
        assert_eq!(transform.details[1][0], 0);
    }

    #[test]
    fn test_level_energy() {
        let details = [3, 4, 0, 0];
        let energy = level_energy(&details, 2);
        assert_eq!(energy, 9 + 16); // 3² + 4²
    }
}
