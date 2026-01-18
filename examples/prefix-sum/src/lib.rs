//! Parallel prefix sum (scan) implementation.
//!
//! Implements the Blelloch scan algorithm which naturally decomposes
//! into up-sweep and down-sweep phases that can be parallelized.

#![no_std]

/// Compute sequential inclusive prefix sum: out[i] = sum(arr[0..=i])
pub fn prefix_sum_sequential(arr: &[u64], out: &mut [u64]) {
    if arr.is_empty() {
        return;
    }

    out[0] = arr[0];
    for i in 1..arr.len() {
        out[i] = out[i - 1].wrapping_add(arr[i]);
    }
}

/// Compute exclusive prefix sum: out[i] = sum(arr[0..i])
pub fn prefix_sum_exclusive(arr: &[u64], out: &mut [u64]) {
    if arr.is_empty() {
        return;
    }

    out[0] = 0;
    for i in 1..arr.len() {
        out[i] = out[i - 1].wrapping_add(arr[i - 1]);
    }
}

/// Parallel-friendly prefix sum using block decomposition.
///
/// Phase 1: Compute local prefix sums within each block (parallelizable)
/// Phase 2: Compute block totals prefix sum (sequential)
/// Phase 3: Add block offsets to each element (parallelizable)
pub fn prefix_sum_blocked(arr: &[u64], out: &mut [u64], num_blocks: usize) {
    let n = arr.len();
    if n == 0 {
        return;
    }

    let block_size = (n + num_blocks - 1) / num_blocks;

    // Phase 1: Compute local prefix sums within each block
    // This phase is embarrassingly parallel
    for block in 0..num_blocks {
        let start = block * block_size;
        let end = core::cmp::min(start + block_size, n);

        if start < n {
            // Local prefix sum for this block
            out[start] = arr[start];
            for i in (start + 1)..end {
                out[i] = out[i - 1].wrapping_add(arr[i]);
            }
        }
    }

    // Phase 2: Compute prefix sum of block totals (sequential)
    // block_totals[i] = sum of all elements in blocks 0..i
    let mut block_offsets = [0u64; 32]; // Support up to 32 blocks
    let mut running_total = 0u64;
    for block in 0..num_blocks {
        block_offsets[block] = running_total;

        let start = block * block_size;
        let end = core::cmp::min(start + block_size, n);
        if end > start {
            running_total = running_total.wrapping_add(out[end - 1]);
        }
    }

    // Phase 3: Add block offsets to each element (parallelizable)
    for block in 1..num_blocks {
        let start = block * block_size;
        let end = core::cmp::min(start + block_size, n);
        let offset = block_offsets[block];

        for i in start..end {
            out[i] = out[i].wrapping_add(offset);
        }
    }
}

/// Verify prefix sum correctness
pub fn verify_prefix_sum(arr: &[u64], prefix: &[u64]) -> bool {
    if arr.is_empty() {
        return prefix.is_empty();
    }

    let mut expected = arr[0];
    if prefix[0] != expected {
        return false;
    }

    for i in 1..arr.len() {
        expected = expected.wrapping_add(arr[i]);
        if prefix[i] != expected {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_prefix_sum() {
        let arr = [1, 2, 3, 4, 5];
        let mut out = [0u64; 5];
        prefix_sum_sequential(&arr, &mut out);
        assert_eq!(out, [1, 3, 6, 10, 15]);
    }

    #[test]
    fn test_exclusive_prefix_sum() {
        let arr = [1, 2, 3, 4, 5];
        let mut out = [0u64; 5];
        prefix_sum_exclusive(&arr, &mut out);
        assert_eq!(out, [0, 1, 3, 6, 10]);
    }

    #[test]
    fn test_blocked_equals_sequential() {
        let arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut out_seq = [0u64; 12];
        let mut out_blk = [0u64; 12];

        prefix_sum_sequential(&arr, &mut out_seq);
        prefix_sum_blocked(&arr, &mut out_blk, 4);

        assert_eq!(out_seq, out_blk);
    }
}
