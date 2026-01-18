//! Parallel Merkle Tree Construction
//!
//! Level-by-level tree construction suitable for parallel execution.
//! Each level's hash computations are independent.

#![no_std]

/// Maximum tree depth (supports up to 2^16 = 65536 leaves)
pub const MAX_DEPTH: usize = 16;
/// Maximum number of leaves
pub const MAX_LEAVES: usize = 1 << MAX_DEPTH;
/// Hash output size (256 bits = 32 bytes)
pub const HASH_SIZE: usize = 32;

/// Simple hash type (32-byte array)
pub type Hash = [u8; HASH_SIZE];

/// Zero hash constant
pub const ZERO_HASH: Hash = [0u8; HASH_SIZE];

/// Simple hash function (for demonstration).
/// Uses a simplified mixing algorithm - replace with Keccak/SHA256 for production.
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut result = [0u8; HASH_SIZE];

    // Simple mixing: XOR, rotate, and add
    for i in 0..HASH_SIZE {
        // Mix left and right with position-dependent rotation
        let l = left[i];
        let r = right[(i + 7) % HASH_SIZE];
        let mixed = l.wrapping_add(r).wrapping_add(i as u8);

        // Additional mixing pass
        result[i] = mixed.rotate_left(3) ^ left[(i + 13) % HASH_SIZE];
    }

    // Second pass for better avalanche
    for i in 0..HASH_SIZE {
        result[i] = result[i]
            .wrapping_add(result[(i + 1) % HASH_SIZE])
            .rotate_right(2);
    }

    result
}

/// Hash a single leaf value.
fn hash_leaf(data: &[u8]) -> Hash {
    let mut result = [0u8; HASH_SIZE];

    // Domain separation prefix for leaves
    result[0] = 0x00;

    // Simple hash of input data
    for (i, &byte) in data.iter().enumerate() {
        let idx = (i % (HASH_SIZE - 1)) + 1;
        result[idx] = result[idx].wrapping_add(byte).rotate_left(i as u32 % 8);
    }

    // Finalization mixing
    for i in 0..HASH_SIZE {
        result[i] = result[i] ^ result[(i + 17) % HASH_SIZE];
    }

    result
}

/// Merkle tree node containing hash.
#[derive(Clone, Copy)]
pub struct Node {
    pub hash: Hash,
}

impl Node {
    pub const fn empty() -> Self {
        Self { hash: ZERO_HASH }
    }

    pub fn from_hash(hash: Hash) -> Self {
        Self { hash }
    }

    pub fn from_data(data: &[u8]) -> Self {
        Self {
            hash: hash_leaf(data),
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::empty()
    }
}

/// Merkle tree with level-by-level storage.
/// Supports parallel construction at each level.
pub struct MerkleTree<const N: usize> {
    /// Tree levels: level[0] = leaves, level[depth] = root
    /// Each level i has N / 2^i nodes
    levels: [[Node; N]; MAX_DEPTH + 1],
    /// Number of leaves (must be power of 2)
    pub num_leaves: usize,
    /// Tree depth
    pub depth: usize,
}

impl<const N: usize> MerkleTree<N> {
    pub fn new() -> Self {
        Self {
            levels: [[Node::empty(); N]; MAX_DEPTH + 1],
            num_leaves: 0,
            depth: 0,
        }
    }

    /// Build tree from leaf data.
    /// Parallel-friendly: each level can be computed independently.
    pub fn build(&mut self, leaves: &[Hash]) {
        let n = leaves.len();
        assert!(n.is_power_of_two() && n <= N);

        self.num_leaves = n;
        // Use integer log2 via trailing_zeros (n is power of 2)
        self.depth = n.trailing_zeros() as usize;

        // Level 0: copy leaves
        for (i, hash) in leaves.iter().enumerate() {
            self.levels[0][i] = Node::from_hash(*hash);
        }

        // Build each level from the previous
        // TODO: Each level's hash computations are independent
        let mut level_size = n;
        for level in 1..=self.depth {
            level_size /= 2;

            // Each pair computation is independent (parallel-friendly)
            for i in 0..level_size {
                let left = &self.levels[level - 1][i * 2].hash;
                let right = &self.levels[level - 1][i * 2 + 1].hash;
                self.levels[level][i] = Node::from_hash(hash_pair(left, right));
            }
        }
    }

    /// Get the root hash.
    pub fn root(&self) -> Hash {
        self.levels[self.depth][0].hash
    }

    /// Generate Merkle proof for leaf at index.
    pub fn proof(&self, leaf_index: usize) -> MerkleProof {
        assert!(leaf_index < self.num_leaves);

        let mut proof = MerkleProof::new();
        proof.leaf_index = leaf_index;
        proof.depth = self.depth;

        let mut idx = leaf_index;

        for level in 0..self.depth {
            // Sibling is the other child of our parent
            let sibling_idx = idx ^ 1; // XOR with 1 flips last bit
            proof.siblings[level] = self.levels[level][sibling_idx].hash;
            idx /= 2;
        }

        proof
    }

    /// Get node at specific position.
    pub fn get_node(&self, level: usize, index: usize) -> &Node {
        &self.levels[level][index]
    }
}

impl<const N: usize> Default for MerkleTree<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Merkle proof containing sibling hashes.
#[derive(Clone)]
pub struct MerkleProof {
    /// Sibling hashes from leaf to root (excluding root)
    pub siblings: [Hash; MAX_DEPTH],
    /// Index of the leaf being proved
    pub leaf_index: usize,
    /// Depth of the tree
    pub depth: usize,
}

impl MerkleProof {
    pub fn new() -> Self {
        Self {
            siblings: [ZERO_HASH; MAX_DEPTH],
            leaf_index: 0,
            depth: 0,
        }
    }

    /// Verify proof against expected root.
    pub fn verify(&self, leaf: &Hash, expected_root: &Hash) -> bool {
        let mut current = *leaf;
        let mut idx = self.leaf_index;

        for level in 0..self.depth {
            let sibling = &self.siblings[level];

            // Order depends on whether we're left or right child
            current = if idx % 2 == 0 {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };

            idx /= 2;
        }

        current == *expected_root
    }
}

impl Default for MerkleProof {
    fn default() -> Self {
        Self::new()
    }
}

/// Build multiple Merkle trees in batch.
/// Each tree is completely independent (embarrassingly parallel).
pub fn batch_build<const N: usize>(
    leaf_sets: &[[Hash; N]],
    trees: &mut [MerkleTree<N>],
    leaf_count: usize,
) {
    assert_eq!(leaf_sets.len(), trees.len());

    // Each tree can be built by a different thread
    for (leaves, tree) in leaf_sets.iter().zip(trees.iter_mut()) {
        tree.build(&leaves[..leaf_count]);
    }
}

/// Verify multiple proofs in batch.
/// Each verification is independent (embarrassingly parallel).
pub fn batch_verify(
    proofs: &[MerkleProof],
    leaves: &[Hash],
    roots: &[Hash],
    results: &mut [bool],
) {
    assert_eq!(proofs.len(), leaves.len());
    assert_eq!(proofs.len(), roots.len());
    assert_eq!(proofs.len(), results.len());

    // Each verification can be done by a different thread
    for (i, proof) in proofs.iter().enumerate() {
        results[i] = proof.verify(&leaves[i], &roots[i]);
    }
}

/// Compute hashes for a level in parallel preparation.
/// Returns array of hash pairs that can be computed independently.
pub fn prepare_level_hashes<const N: usize>(
    prev_level: &[Node; N],
    level_size: usize,
) -> [(Hash, Hash); N] {
    let mut pairs = [(ZERO_HASH, ZERO_HASH); N];

    for i in 0..level_size {
        pairs[i] = (
            prev_level[i * 2].hash,
            prev_level[i * 2 + 1].hash,
        );
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_leaf(value: u8) -> Hash {
        let mut h = ZERO_HASH;
        h[0] = value;
        hash_leaf(&h)
    }

    #[test]
    fn test_hash_pair_deterministic() {
        let a = [1u8; HASH_SIZE];
        let b = [2u8; HASH_SIZE];

        let h1 = hash_pair(&a, &b);
        let h2 = hash_pair(&a, &b);

        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_pair_order_matters() {
        let a = [1u8; HASH_SIZE];
        let b = [2u8; HASH_SIZE];

        let h1 = hash_pair(&a, &b);
        let h2 = hash_pair(&b, &a);

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_simple_tree() {
        let leaves = [make_leaf(1), make_leaf(2), make_leaf(3), make_leaf(4)];

        let mut tree = MerkleTree::<4>::new();
        tree.build(&leaves);

        assert_eq!(tree.num_leaves, 4);
        assert_eq!(tree.depth, 2);

        // Root should be non-zero
        assert_ne!(tree.root(), ZERO_HASH);
    }

    #[test]
    fn test_proof_verification() {
        let leaves = [make_leaf(1), make_leaf(2), make_leaf(3), make_leaf(4)];

        let mut tree = MerkleTree::<4>::new();
        tree.build(&leaves);

        let root = tree.root();

        // Verify proof for each leaf
        for i in 0..4 {
            let proof = tree.proof(i);
            assert!(proof.verify(&leaves[i], &root));
        }
    }

    #[test]
    fn test_invalid_proof() {
        let leaves = [make_leaf(1), make_leaf(2), make_leaf(3), make_leaf(4)];

        let mut tree = MerkleTree::<4>::new();
        tree.build(&leaves);

        let root = tree.root();
        let proof = tree.proof(0);

        // Wrong leaf should fail
        let wrong_leaf = make_leaf(99);
        assert!(!proof.verify(&wrong_leaf, &root));
    }

    #[test]
    fn test_tree_determinism() {
        let leaves = [make_leaf(1), make_leaf(2), make_leaf(3), make_leaf(4)];

        let mut tree1 = MerkleTree::<4>::new();
        let mut tree2 = MerkleTree::<4>::new();

        tree1.build(&leaves);
        tree2.build(&leaves);

        assert_eq!(tree1.root(), tree2.root());
    }
}
