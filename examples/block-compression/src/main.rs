//! Block Compression Example
//!
//! Demonstrates parallel block-wise compression.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use block_compression::{
    compress_blocks, delta_decode, delta_encode, estimate_entropy, histogram,
    lz77_compress, lz77_decompress, rle_compress, rle_decompress,
    Lz77Block, RleBlock, BLOCK_SIZE,
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
    println!("=== Block Compression Example ===");

    // Test 1: RLE with repetitive data
    println!("\nTest 1: RLE Compression");
    let repetitive: [u8; 32] = [
        1, 1, 1, 1, 1, 1, 1, 1,  // 8 ones
        2, 2, 2, 2,              // 4 twos
        3, 3, 3, 3, 3, 3,        // 6 threes
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, // 14 fours
    ];

    let rle = rle_compress(&repetitive);
    println!("  Original size: {} bytes", repetitive.len());
    println!("  RLE pairs: {}", rle.len);
    println!("  Compressed size: {} bytes (pairs * 2)", rle.len * 2);

    // Verify roundtrip
    let mut rle_output = [0u8; 64];
    let rle_len = rle_decompress(&rle, &mut rle_output);
    let rle_match = rle_output[..rle_len] == repetitive;
    println!("  Roundtrip: {}", if rle_match { "PASS" } else { "FAIL" });

    // Test 2: RLE with non-repetitive data
    println!("\nTest 2: RLE on Random Data");
    let random: [u8; 16] = [1, 5, 2, 8, 3, 7, 4, 6, 9, 0, 1, 2, 3, 4, 5, 6];

    let rle_random = rle_compress(&random);
    println!("  Original: {} bytes", random.len());
    println!("  RLE pairs: {} (expansion!)", rle_random.len);
    println!("  RLE not effective for random data");

    // Test 3: LZ77 with repeating pattern
    println!("\nTest 3: LZ77 Compression");
    let pattern: [u8; 24] = [
        1, 2, 3, 4, 5, 6,
        1, 2, 3, 4, 5, 6, // repeat
        1, 2, 3, 4, 5, 6, // repeat
        7, 8, 9, 10, 11, 12,
    ];

    let lz77 = lz77_compress(&pattern);
    println!("  Original size: {} bytes", pattern.len());
    println!("  LZ77 tokens: {}", lz77.len);
    println!("  Approx compressed: {} bytes", lz77.compressed_size());

    // Count match vs literal tokens
    let mut matches = 0;
    let mut literals = 0;
    for token in &lz77.tokens[..lz77.len] {
        match token {
            block_compression::Token::Match { .. } => matches += 1,
            block_compression::Token::Literal(_) => literals += 1,
        }
    }
    println!("  Literals: {}, Matches: {}", literals, matches);

    // Verify roundtrip
    let mut lz77_output = [0u8; 64];
    let lz77_len = lz77_decompress(&lz77, &mut lz77_output);
    let lz77_match = lz77_output[..lz77_len] == pattern;
    println!("  Roundtrip: {}", if lz77_match { "PASS" } else { "FAIL" });

    // Test 4: Delta encoding for gradual data
    println!("\nTest 4: Delta Encoding");
    let gradual: [u8; 8] = [100, 102, 105, 107, 110, 108, 112, 115];
    let mut delta_encoded = [0u8; 8];
    let mut delta_decoded = [0u8; 8];

    delta_encode(&gradual, &mut delta_encoded);
    println!("  Original: {:?}", &gradual[..4]);
    println!("  Delta encoded: {:?}", &delta_encoded[..4]);

    // Delta encoding reduces magnitude, making RLE/LZ77 more effective
    let delta_rle = rle_compress(&delta_encoded);
    println!("  RLE after delta: {} pairs", delta_rle.len);

    delta_decode(&delta_encoded, &mut delta_decoded);
    let delta_match = delta_decoded == gradual;
    println!("  Roundtrip: {}", if delta_match { "PASS" } else { "FAIL" });

    // Test 5: Histogram and entropy
    println!("\nTest 5: Entropy Analysis");

    // Low entropy (repetitive)
    let mut hist = [0u32; 256];
    histogram(&repetitive, &mut hist);
    let entropy_rep = estimate_entropy(&hist, repetitive.len());
    println!("  Repetitive data entropy: ~{} bits/byte", entropy_rep);

    // Higher entropy (more varied)
    histogram(&random, &mut hist);
    let entropy_rand = estimate_entropy(&hist, random.len());
    println!("  Random data entropy: ~{} bits/byte", entropy_rand);

    // Test 6: Block-based compression (parallel-friendly)
    println!("\nTest 6: Block-Based Compression");

    // Create data with different characteristics per block
    let mut multiblock = [0u8; 128];
    // Block 0: repetitive
    for i in 0..32 {
        multiblock[i] = 5;
    }
    // Block 1: pattern
    for i in 0..32 {
        multiblock[32 + i] = (i % 4) as u8;
    }
    // Block 2: gradual
    for i in 0..32 {
        multiblock[64 + i] = i as u8;
    }
    // Block 3: mixed
    for i in 0..32 {
        multiblock[96 + i] = ((i * 7) % 256) as u8;
    }

    let mut rle_blocks = [RleBlock::new(), RleBlock::new(), RleBlock::new(), RleBlock::new()];
    let mut lz77_blocks = [Lz77Block::new(), Lz77Block::new(), Lz77Block::new(), Lz77Block::new()];

    let num_blocks = compress_blocks(&multiblock, 32, &mut rle_blocks, &mut lz77_blocks);
    println!("  Total blocks: {}", num_blocks);

    for i in 0..num_blocks {
        println!("  Block {}: RLE={} pairs, LZ77={} tokens",
                 i, rle_blocks[i].len, lz77_blocks[i].len);
    }

    // Test 7: Compression comparison
    println!("\nTest 7: Algorithm Comparison");

    // Test different data patterns
    let patterns: [(&str, [u8; 16]); 4] = [
        ("All zeros", [0; 16]),
        ("Ascending", [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        ("Repeating", [1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2]),
        ("Mixed", [1, 1, 1, 2, 3, 4, 4, 4, 5, 6, 6, 7, 8, 8, 8, 8]),
    ];

    for (name, data) in &patterns {
        let rle = rle_compress(data);
        let lz77 = lz77_compress(data);
        println!("  {}: RLE={} pairs, LZ77={} tokens",
                 name, rle.len, lz77.len);
    }

    // Test 8: Verify block independence
    println!("\nTest 8: Block Independence Verification");
    let block_a: [u8; 8] = [1, 1, 1, 1, 2, 2, 2, 2];
    let block_b: [u8; 8] = [3, 3, 3, 3, 4, 4, 4, 4];

    // Compress separately
    let rle_a = rle_compress(&block_a);
    let rle_b = rle_compress(&block_b);

    // Compress together
    let mut combined = [0u8; 16];
    combined[..8].copy_from_slice(&block_a);
    combined[8..].copy_from_slice(&block_b);

    let mut rle_combined = [RleBlock::new(), RleBlock::new()];
    let mut lz_combined = [Lz77Block::new(), Lz77Block::new()];
    compress_blocks(&combined, 8, &mut rle_combined, &mut lz_combined);

    let independent = rle_a.len == rle_combined[0].len && rle_b.len == rle_combined[1].len;
    println!("  Blocks compressed independently: {}", if independent { "PASS" } else { "FAIL" });

    println!("\n=== Block Compression Example Complete ===");

    platform::exit(0)
}
