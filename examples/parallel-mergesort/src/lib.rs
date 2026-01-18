//! Merge sort implementation with parallel-friendly structure.
//!
//! The algorithm naturally decomposes into independent sub-problems,
//! making it ideal for demonstrating thread-based parallelism.

#![no_std]

/// Merge two sorted slices into the output buffer
pub fn merge(left: &[u32], right: &[u32], output: &mut [u32]) {
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            output[k] = left[i];
            i += 1;
        } else {
            output[k] = right[j];
            j += 1;
        }
        k += 1;
    }

    // Copy remaining elements from left
    while i < left.len() {
        output[k] = left[i];
        i += 1;
        k += 1;
    }

    // Copy remaining elements from right
    while j < right.len() {
        output[k] = right[j];
        j += 1;
        k += 1;
    }
}

/// Single-threaded merge sort (in-place using auxiliary buffer)
pub fn merge_sort(arr: &mut [u32], aux: &mut [u32]) {
    let n = arr.len();
    if n <= 1 {
        return;
    }

    let mid = n / 2;

    // Recursively sort halves
    merge_sort(&mut arr[..mid], &mut aux[..mid]);
    merge_sort(&mut arr[mid..], &mut aux[mid..]);

    // Merge into auxiliary buffer
    merge(&arr[..mid], &arr[mid..], aux);

    // Copy back
    arr.copy_from_slice(&aux[..n]);
}

/// Sort independent segments (preparation for parallel merge)
/// Each segment can be sorted by a different thread
pub fn sort_segments(arr: &mut [u32], aux: &mut [u32], num_segments: usize) {
    let n = arr.len();
    let segment_size = (n + num_segments - 1) / num_segments;

    for i in 0..num_segments {
        let start = i * segment_size;
        let end = core::cmp::min(start + segment_size, n);
        if start < n {
            let seg_len = end - start;
            merge_sort(&mut arr[start..end], &mut aux[start..start + seg_len]);
        }
    }
}

/// Merge sorted segments pairwise
pub fn merge_segments(arr: &mut [u32], aux: &mut [u32], num_segments: usize) {
    let n = arr.len();
    let segment_size = (n + num_segments - 1) / num_segments;

    // Pairwise merge until all segments are combined
    let mut current_segments = num_segments;
    let mut current_size = segment_size;

    while current_segments > 1 {
        let pairs = (current_segments + 1) / 2;

        for p in 0..pairs {
            let left_start = p * 2 * current_size;
            let left_end = core::cmp::min(left_start + current_size, n);
            let right_start = left_end;
            let right_end = core::cmp::min(right_start + current_size, n);

            if right_start < n {
                // Merge two adjacent segments
                merge(
                    &arr[left_start..left_end],
                    &arr[right_start..right_end],
                    &mut aux[left_start..right_end],
                );
                arr[left_start..right_end].copy_from_slice(&aux[left_start..right_end]);
            }
        }

        current_segments = pairs;
        current_size *= 2;
    }
}

/// Check if array is sorted
pub fn is_sorted(arr: &[u32]) -> bool {
    for i in 1..arr.len() {
        if arr[i - 1] > arr[i] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let left = [1, 3, 5, 7];
        let right = [2, 4, 6, 8];
        let mut output = [0u32; 8];
        merge(&left, &right, &mut output);
        assert_eq!(output, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_sort() {
        let mut arr = [5, 2, 8, 1, 9, 3, 7, 4, 6];
        let mut aux = [0u32; 9];
        merge_sort(&mut arr, &mut aux);
        assert!(is_sorted(&arr));
    }

    #[test]
    fn test_sort_segments() {
        let mut arr = [8, 4, 2, 6, 1, 5, 3, 7];
        let mut aux = [0u32; 8];
        sort_segments(&mut arr, &mut aux, 4);
        // Each segment of 2 should be sorted
        assert!(arr[0] <= arr[1]);
        assert!(arr[2] <= arr[3]);
        assert!(arr[4] <= arr[5]);
        assert!(arr[6] <= arr[7]);
    }
}
