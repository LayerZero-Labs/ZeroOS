//! Matrix multiplication implementation with block-parallel structure.
//!
//! Matrices are stored in row-major order. The computation is structured
//! so that row blocks can be computed independently by different threads.

#![no_std]

/// Matrix dimension (NxN matrices)
pub const DIM: usize = 16;

/// Matrix type: DIM x DIM array of i32
pub type Matrix = [[i32; DIM]; DIM];

/// Initialize a matrix with zeros
pub fn zero_matrix() -> Matrix {
    [[0i32; DIM]; DIM]
}

/// Initialize a matrix with deterministic test values
pub fn init_matrix(seed: u32) -> Matrix {
    let mut m = zero_matrix();
    let mut s = seed;
    for i in 0..DIM {
        for j in 0..DIM {
            // Simple LCG for deterministic values
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            m[i][j] = ((s >> 16) % 100) as i32;
        }
    }
    m
}

/// Standard matrix multiplication: C = A * B
pub fn matmul(a: &Matrix, b: &Matrix, c: &mut Matrix) {
    for i in 0..DIM {
        for j in 0..DIM {
            let mut sum = 0i32;
            for k in 0..DIM {
                sum = sum.wrapping_add(a[i][k].wrapping_mul(b[k][j]));
            }
            c[i][j] = sum;
        }
    }
}

/// Compute a single row block of the result matrix.
/// This function computes rows [start_row, end_row) of C = A * B.
/// Can be called independently by different threads.
pub fn matmul_row_block(
    a: &Matrix,
    b: &Matrix,
    c: &mut Matrix,
    start_row: usize,
    end_row: usize,
) {
    let end = core::cmp::min(end_row, DIM);
    for i in start_row..end {
        for j in 0..DIM {
            let mut sum = 0i32;
            for k in 0..DIM {
                sum = sum.wrapping_add(a[i][k].wrapping_mul(b[k][j]));
            }
            c[i][j] = sum;
        }
    }
}

/// Parallel-friendly block multiplication.
/// Divides the computation into `num_blocks` row blocks.
pub fn matmul_blocked(a: &Matrix, b: &Matrix, c: &mut Matrix, num_blocks: usize) {
    let rows_per_block = (DIM + num_blocks - 1) / num_blocks;

    for block in 0..num_blocks {
        let start_row = block * rows_per_block;
        let end_row = core::cmp::min(start_row + rows_per_block, DIM);
        matmul_row_block(a, b, c, start_row, end_row);
    }
}

/// Compute checksum of a matrix (for verification)
pub fn matrix_checksum(m: &Matrix) -> i64 {
    let mut sum = 0i64;
    for i in 0..DIM {
        for j in 0..DIM {
            sum = sum.wrapping_add(m[i][j] as i64);
        }
    }
    sum
}

/// Check if two matrices are equal
pub fn matrices_equal(a: &Matrix, b: &Matrix) -> bool {
    for i in 0..DIM {
        for j in 0..DIM {
            if a[i][j] != b[i][j] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matmul_identity() {
        let mut identity = zero_matrix();
        for i in 0..DIM {
            identity[i][i] = 1;
        }

        let a = init_matrix(42);
        let mut c = zero_matrix();
        matmul(&a, &identity, &mut c);

        assert!(matrices_equal(&a, &c));
    }

    #[test]
    fn test_blocked_equals_standard() {
        let a = init_matrix(123);
        let b = init_matrix(456);

        let mut c_std = zero_matrix();
        let mut c_blk = zero_matrix();

        matmul(&a, &b, &mut c_std);
        matmul_blocked(&a, &b, &mut c_blk, 4);

        assert!(matrices_equal(&c_std, &c_blk));
    }
}
