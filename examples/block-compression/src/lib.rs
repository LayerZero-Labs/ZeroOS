//! Parallel Block Compression Implementation
//!
//! Block-wise LZ77 and RLE compression for parallel execution.
//! Each block can be compressed independently.

#![no_std]

/// Maximum block size for compression
pub const BLOCK_SIZE: usize = 256;
/// Maximum output size (worst case: slight expansion)
pub const MAX_OUTPUT: usize = BLOCK_SIZE + 64;
/// Maximum match length for LZ77
pub const MAX_MATCH_LEN: usize = 15;
/// Maximum look-back distance
pub const MAX_DISTANCE: usize = 255;

/// Compression token types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    /// Literal byte
    Literal(u8),
    /// Match reference: (distance back, length)
    Match { distance: u8, length: u8 },
}

/// RLE-compressed block
#[derive(Clone)]
pub struct RleBlock {
    /// Compressed data: (count, value) pairs
    pub data: [(u8, u8); MAX_OUTPUT],
    /// Number of pairs used
    pub len: usize,
    /// Original uncompressed size
    pub original_size: usize,
}

impl RleBlock {
    pub fn new() -> Self {
        Self {
            data: [(0, 0); MAX_OUTPUT],
            len: 0,
            original_size: 0,
        }
    }

    /// Compression ratio (compressed / original)
    pub fn ratio(&self) -> f32 {
        if self.original_size == 0 {
            return 1.0;
        }
        (self.len * 2) as f32 / self.original_size as f32
    }
}

impl Default for RleBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// LZ77-compressed block
#[derive(Clone)]
pub struct Lz77Block {
    /// Compressed tokens
    pub tokens: [Token; MAX_OUTPUT],
    /// Number of tokens
    pub len: usize,
    /// Original size
    pub original_size: usize,
}

impl Lz77Block {
    pub fn new() -> Self {
        Self {
            tokens: [Token::Literal(0); MAX_OUTPUT],
            len: 0,
            original_size: 0,
        }
    }

    /// Approximate compressed size in bytes
    pub fn compressed_size(&self) -> usize {
        let mut size = 0;
        for token in &self.tokens[..self.len] {
            match token {
                Token::Literal(_) => size += 2, // flag + byte
                Token::Match { .. } => size += 2, // distance + length
            }
        }
        size
    }
}

impl Default for Lz77Block {
    fn default() -> Self {
        Self::new()
    }
}

/// Run-Length Encoding compression.
/// Simple but effective for data with repeated values.
pub fn rle_compress(input: &[u8]) -> RleBlock {
    let mut result = RleBlock::new();
    result.original_size = input.len();

    if input.is_empty() {
        return result;
    }

    let mut i = 0;
    while i < input.len() {
        let value = input[i];
        let mut count = 1u8;

        // Count consecutive identical bytes
        while (i + count as usize) < input.len()
            && input[i + count as usize] == value
            && count < 255
        {
            count += 1;
        }

        result.data[result.len] = (count, value);
        result.len += 1;
        i += count as usize;
    }

    result
}

/// RLE decompression.
pub fn rle_decompress(compressed: &RleBlock, output: &mut [u8]) -> usize {
    let mut pos = 0;

    for i in 0..compressed.len {
        let (count, value) = compressed.data[i];
        for _ in 0..count {
            if pos < output.len() {
                output[pos] = value;
                pos += 1;
            }
        }
    }

    pos
}

/// Find longest match in sliding window for LZ77.
fn find_match(data: &[u8], pos: usize, window_start: usize) -> Option<(u8, u8)> {
    if pos >= data.len() {
        return None;
    }

    let mut best_distance = 0u8;
    let mut best_length = 0u8;

    // Search backwards in window
    let search_start = if pos > MAX_DISTANCE {
        pos - MAX_DISTANCE
    } else {
        window_start
    };

    for match_pos in search_start..pos {
        let mut length = 0usize;

        // Count matching bytes
        while pos + length < data.len()
            && data[match_pos + length] == data[pos + length]
            && length < MAX_MATCH_LEN
        {
            length += 1;
        }

        if length > best_length as usize && length >= 3 {
            best_distance = (pos - match_pos) as u8;
            best_length = length as u8;
        }
    }

    if best_length >= 3 {
        Some((best_distance, best_length))
    } else {
        None
    }
}

/// LZ77 compression with sliding window.
pub fn lz77_compress(input: &[u8]) -> Lz77Block {
    let mut result = Lz77Block::new();
    result.original_size = input.len();

    let mut pos = 0;

    while pos < input.len() {
        if let Some((distance, length)) = find_match(input, pos, 0) {
            result.tokens[result.len] = Token::Match { distance, length };
            pos += length as usize;
        } else {
            result.tokens[result.len] = Token::Literal(input[pos]);
            pos += 1;
        }
        result.len += 1;
    }

    result
}

/// LZ77 decompression.
pub fn lz77_decompress(compressed: &Lz77Block, output: &mut [u8]) -> usize {
    let mut pos = 0;

    for i in 0..compressed.len {
        match compressed.tokens[i] {
            Token::Literal(byte) => {
                if pos < output.len() {
                    output[pos] = byte;
                    pos += 1;
                }
            }
            Token::Match { distance, length } => {
                let start = pos - distance as usize;
                for j in 0..length as usize {
                    if pos < output.len() {
                        output[pos] = output[start + j];
                        pos += 1;
                    }
                }
            }
        }
    }

    pos
}

/// Block-based compression (parallel-friendly).
/// Each block is compressed independently.
pub fn compress_blocks(
    input: &[u8],
    block_size: usize,
    rle_blocks: &mut [RleBlock],
    lz77_blocks: &mut [Lz77Block],
) -> usize {
    let num_blocks = (input.len() + block_size - 1) / block_size;

    // Each block can be compressed independently by a different thread
    for i in 0..num_blocks {
        let start = i * block_size;
        let end = core::cmp::min(start + block_size, input.len());
        let block = &input[start..end];

        rle_blocks[i] = rle_compress(block);
        lz77_blocks[i] = lz77_compress(block);
    }

    num_blocks
}

/// Simple byte histogram (useful for entropy estimation).
pub fn histogram(data: &[u8], hist: &mut [u32; 256]) {
    for h in hist.iter_mut() {
        *h = 0;
    }
    for &byte in data {
        hist[byte as usize] += 1;
    }
}

/// Estimate entropy (bits per byte) from histogram.
/// Lower entropy = more compressible.
pub fn estimate_entropy(hist: &[u32; 256], total: usize) -> u32 {
    if total == 0 {
        return 0;
    }

    // Fixed-point entropy calculation (scaled by 1000)
    let mut entropy: u64 = 0;
    let total_u64 = total as u64;

    for &count in hist.iter() {
        if count > 0 {
            let p = (count as u64 * 1000) / total_u64;
            if p > 0 {
                // Approximate -p*log2(p) using lookup or approximation
                // log2(p/1000) ≈ log2(p) - 10
                // Simple approximation: entropy contribution ~ p * (10 - log2(p))
                let log_approx = 10 - (64 - p.leading_zeros()) as u64;
                entropy += p * log_approx;
            }
        }
    }

    (entropy / 1000) as u32
}

/// Delta encoding (for sequences with gradual changes).
/// Encodes differences between consecutive values.
pub fn delta_encode(input: &[u8], output: &mut [u8]) -> usize {
    if input.is_empty() {
        return 0;
    }

    output[0] = input[0];
    for i in 1..input.len() {
        output[i] = input[i].wrapping_sub(input[i - 1]);
    }
    input.len()
}

/// Delta decoding.
pub fn delta_decode(input: &[u8], output: &mut [u8]) -> usize {
    if input.is_empty() {
        return 0;
    }

    output[0] = input[0];
    for i in 1..input.len() {
        output[i] = output[i - 1].wrapping_add(input[i]);
    }
    input.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_simple() {
        let input = [1, 1, 1, 1, 2, 2, 3];
        let compressed = rle_compress(&input);

        assert_eq!(compressed.len, 3);
        assert_eq!(compressed.data[0], (4, 1));
        assert_eq!(compressed.data[1], (2, 2));
        assert_eq!(compressed.data[2], (1, 3));
    }

    #[test]
    fn test_rle_roundtrip() {
        let input = [5, 5, 5, 10, 10, 15, 15, 15, 15];
        let compressed = rle_compress(&input);

        let mut output = [0u8; 16];
        let len = rle_decompress(&compressed, &mut output);

        assert_eq!(&output[..len], &input);
    }

    #[test]
    fn test_lz77_literal() {
        // No repeats = all literals
        let input = [1, 2, 3, 4, 5];
        let compressed = lz77_compress(&input);

        assert_eq!(compressed.len, 5);
        for (i, token) in compressed.tokens[..5].iter().enumerate() {
            assert_eq!(*token, Token::Literal(input[i]));
        }
    }

    #[test]
    fn test_lz77_match() {
        // Repeated pattern should create matches
        let input = [1, 2, 3, 1, 2, 3, 1, 2, 3];
        let compressed = lz77_compress(&input);

        // First 3 bytes are literals, then matches
        assert!(compressed.len < input.len());
    }

    #[test]
    fn test_lz77_roundtrip() {
        let input = [1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 5, 6, 5, 6];
        let compressed = lz77_compress(&input);

        let mut output = [0u8; 32];
        let len = lz77_decompress(&compressed, &mut output);

        assert_eq!(&output[..len], &input);
    }

    #[test]
    fn test_delta_encoding() {
        let input = [10, 12, 15, 14, 16];
        let mut encoded = [0u8; 5];
        let mut decoded = [0u8; 5];

        delta_encode(&input, &mut encoded);
        assert_eq!(encoded, [10, 2, 3, 255, 2]); // 255 = -1 as u8

        delta_decode(&encoded, &mut decoded);
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_histogram() {
        let data = [1, 1, 1, 2, 2, 3];
        let mut hist = [0u32; 256];
        histogram(&data, &mut hist);

        assert_eq!(hist[1], 3);
        assert_eq!(hist[2], 2);
        assert_eq!(hist[3], 1);
        assert_eq!(hist[0], 0);
    }
}
