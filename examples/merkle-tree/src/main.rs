//! Merkle Tree Example
//!
//! Demonstrates parallel Merkle tree construction and proof verification.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use merkle_tree::{batch_verify, Hash, MerkleProof, MerkleTree, ZERO_HASH};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Create a test leaf hash from a simple value.
fn make_leaf(value: u8) -> Hash {
    let mut h = ZERO_HASH;
    h[0] = value;
    // Simple additional mixing
    for i in 1..32 {
        h[i] = h[i - 1].wrapping_add(value).wrapping_mul(17);
    }
    h
}

/// Print first few bytes of a hash.
fn print_hash_prefix(h: &Hash) {
    println!(
        "    {:02x}{:02x}{:02x}{:02x}...",
        h[0], h[1], h[2], h[3]
    );
}

#[unsafe(no_mangle)]
fn main() -> ! {
    println!("=== Merkle Tree Example ===");

    // Test 1: Simple 4-leaf tree
    println!("\nTest 1: 4-Leaf Tree");
    let leaves_4 = [make_leaf(1), make_leaf(2), make_leaf(3), make_leaf(4)];

    let mut tree_4 = MerkleTree::<4>::new();
    tree_4.build(&leaves_4);

    println!("  Leaves: 4");
    println!("  Depth: {}", tree_4.depth);
    println!("  Root hash:");
    print_hash_prefix(&tree_4.root());

    // Test 2: Proof generation and verification
    println!("\nTest 2: Proof Verification");
    let root = tree_4.root();

    let mut all_valid = true;
    for i in 0..4 {
        let proof = tree_4.proof(i);
        let valid = proof.verify(&leaves_4[i], &root);
        if !valid {
            all_valid = false;
        }
        println!("  Leaf {} proof valid: {}", i, valid);
    }
    println!("  All proofs valid: {}", if all_valid { "PASS" } else { "FAIL" });

    // Test 3: Invalid proof detection
    println!("\nTest 3: Invalid Proof Detection");
    let proof_0 = tree_4.proof(0);
    let wrong_leaf = make_leaf(99);
    let invalid = !proof_0.verify(&wrong_leaf, &root);
    println!("  Wrong leaf rejected: {}", if invalid { "PASS" } else { "FAIL" });

    // Wrong root test
    let wrong_root = make_leaf(88);
    let invalid_root = !proof_0.verify(&leaves_4[0], &wrong_root);
    println!("  Wrong root rejected: {}", if invalid_root { "PASS" } else { "FAIL" });

    // Test 4: Larger tree (16 leaves)
    println!("\nTest 4: 16-Leaf Tree");
    let mut leaves_16 = [ZERO_HASH; 16];
    for i in 0..16 {
        leaves_16[i] = make_leaf(i as u8);
    }

    let mut tree_16 = MerkleTree::<16>::new();
    tree_16.build(&leaves_16);

    println!("  Leaves: 16");
    println!("  Depth: {}", tree_16.depth);
    println!("  Root hash:");
    print_hash_prefix(&tree_16.root());

    // Verify random leaf
    let proof_10 = tree_16.proof(10);
    let valid_10 = proof_10.verify(&leaves_16[10], &tree_16.root());
    println!("  Proof for leaf 10: {}", if valid_10 { "PASS" } else { "FAIL" });

    // Test 5: Tree determinism
    println!("\nTest 5: Determinism");
    let mut tree_16_copy = MerkleTree::<16>::new();
    tree_16_copy.build(&leaves_16);

    let deterministic = tree_16.root() == tree_16_copy.root();
    println!("  Same input -> same root: {}", if deterministic { "PASS" } else { "FAIL" });

    // Test 6: Different leaves produce different roots
    println!("\nTest 6: Collision Resistance");
    let mut leaves_different = leaves_16;
    leaves_different[0] = make_leaf(255); // Change one leaf

    let mut tree_different = MerkleTree::<16>::new();
    tree_different.build(&leaves_different);

    let no_collision = tree_16.root() != tree_different.root();
    println!("  One changed leaf changes root: {}", if no_collision { "PASS" } else { "FAIL" });

    // Test 7: Batch proof verification (parallel-friendly)
    println!("\nTest 7: Batch Verification (parallel-friendly)");
    let proofs: [MerkleProof; 4] = [
        tree_16.proof(0),
        tree_16.proof(5),
        tree_16.proof(10),
        tree_16.proof(15),
    ];
    let batch_leaves = [leaves_16[0], leaves_16[5], leaves_16[10], leaves_16[15]];
    let batch_roots = [tree_16.root(), tree_16.root(), tree_16.root(), tree_16.root()];
    let mut results = [false; 4];

    batch_verify(&proofs, &batch_leaves, &batch_roots, &mut results);

    let batch_ok = results.iter().all(|&r| r);
    println!("  Verified 4 proofs in batch");
    println!("  All valid: {}", if batch_ok { "PASS" } else { "FAIL" });

    // Test 8: Proof size analysis
    println!("\nTest 8: Proof Size Analysis");
    let proof_size_bits = tree_16.depth * 256; // Each sibling is 256 bits
    let proof_size_bytes = tree_16.depth * 32;
    println!("  Tree depth: {}", tree_16.depth);
    println!("  Proof size: {} siblings = {} bits = {} bytes",
             tree_16.depth, proof_size_bits, proof_size_bytes);

    // Test 9: Level-by-level inspection
    println!("\nTest 9: Tree Structure");
    println!("  Level 0 (leaves): {} nodes", tree_16.num_leaves);
    let mut level_size = tree_16.num_leaves;
    for level in 1..=tree_16.depth {
        level_size /= 2;
        println!("  Level {} (internal): {} nodes", level, level_size);
    }

    // Show a path through the tree
    println!("\n  Path for leaf 5:");
    let mut idx = 5usize;
    for level in 0..=tree_16.depth {
        let node = tree_16.get_node(level, idx);
        println!("    Level {}, idx {}: {:02x}{:02x}{:02x}{:02x}...",
                 level, idx, node.hash[0], node.hash[1], node.hash[2], node.hash[3]);
        idx /= 2;
    }

    // Test 10: Edge case - 2 leaves (minimum tree)
    println!("\nTest 10: Minimum Tree (2 leaves)");
    let leaves_2 = [make_leaf(100), make_leaf(200)];
    let mut tree_2 = MerkleTree::<2>::new();
    tree_2.build(&leaves_2);

    println!("  Depth: {}", tree_2.depth);
    let proof_min = tree_2.proof(0);
    let valid_min = proof_min.verify(&leaves_2[0], &tree_2.root());
    println!("  Proof works: {}", if valid_min { "PASS" } else { "FAIL" });

    println!("\n=== Merkle Tree Example Complete ===");

    platform::exit(0)
}
