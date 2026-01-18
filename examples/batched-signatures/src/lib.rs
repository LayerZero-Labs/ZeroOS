//! Simplified Ed25519-like signature verification.
//!
//! This is a toy implementation for demonstration purposes.
//! In production, use a proper cryptographic library.
//!
//! The implementation focuses on exercising the computation patterns
//! without full cryptographic security.

#![no_std]

/// A simplified "public key" (32 bytes)
pub type PublicKey = [u8; 32];

/// A simplified "signature" (64 bytes)
pub type Signature = [u8; 64];

/// A message to verify
pub type Message<'a> = &'a [u8];

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    Valid,
    Invalid,
}

/// Simple hash function for demonstration (not cryptographically secure!)
fn simple_hash(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut acc: u64 = 0x5555555555555555;

    for (i, &byte) in data.iter().enumerate() {
        acc = acc.wrapping_mul(31).wrapping_add(byte as u64);
        acc ^= acc.rotate_left(13);
        hash[i % 32] ^= (acc & 0xFF) as u8;
        acc = acc.wrapping_add((i as u64).wrapping_mul(17));
    }

    // Final mixing
    for i in 0..32 {
        acc = acc.wrapping_mul(0x5851F42D4C957F2D);
        acc ^= acc >> 33;
        hash[i] ^= (acc & 0xFF) as u8;
    }

    hash
}

/// Generate a deterministic "signature" for testing.
/// This is NOT real Ed25519 - just a demo to exercise computation patterns.
pub fn sign_message(secret_key: &[u8; 32], message: &[u8]) -> Signature {
    let mut sig = [0u8; 64];

    // First 32 bytes: hash of secret_key || message
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(secret_key);
    let msg_len = core::cmp::min(message.len(), 32);
    combined[32..32 + msg_len].copy_from_slice(&message[..msg_len]);

    let r = simple_hash(&combined);
    sig[..32].copy_from_slice(&r);

    // Second 32 bytes: hash of r || public_key || message
    let public_key = derive_public_key(secret_key);
    let mut combined2 = [0u8; 96];
    combined2[..32].copy_from_slice(&r);
    combined2[32..64].copy_from_slice(&public_key);
    let msg_len2 = core::cmp::min(message.len(), 32);
    combined2[64..64 + msg_len2].copy_from_slice(&message[..msg_len2]);

    let s = simple_hash(&combined2);
    sig[32..].copy_from_slice(&s);

    sig
}

/// Derive "public key" from secret key (simplified)
pub fn derive_public_key(secret_key: &[u8; 32]) -> PublicKey {
    simple_hash(secret_key)
}

/// Verify a signature against a public key and message.
/// Returns Valid if the signature matches, Invalid otherwise.
pub fn verify_signature(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> VerifyResult {
    // Reconstruct expected signature components
    let r = &signature[..32];
    let s = &signature[32..];

    // Recompute s' = hash(r || public_key || message)
    let mut combined = [0u8; 96];
    combined[..32].copy_from_slice(r);
    combined[32..64].copy_from_slice(public_key);
    let msg_len = core::cmp::min(message.len(), 32);
    combined[64..64 + msg_len].copy_from_slice(&message[..msg_len]);

    let expected_s = simple_hash(&combined);

    // Check if s matches expected
    if s == expected_s {
        VerifyResult::Valid
    } else {
        VerifyResult::Invalid
    }
}

/// Batch verify multiple signatures (single-threaded baseline)
pub fn batch_verify(
    public_keys: &[PublicKey],
    messages: &[&[u8]],
    signatures: &[Signature],
    results: &mut [VerifyResult],
) {
    let n = core::cmp::min(
        core::cmp::min(public_keys.len(), messages.len()),
        core::cmp::min(signatures.len(), results.len()),
    );

    for i in 0..n {
        results[i] = verify_signature(&public_keys[i], messages[i], &signatures[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let secret_key = [0x42u8; 32];
        let public_key = derive_public_key(&secret_key);
        let message = b"hello world";

        let signature = sign_message(&secret_key, message);
        let result = verify_signature(&public_key, message, &signature);

        assert_eq!(result, VerifyResult::Valid);
    }

    #[test]
    fn test_invalid_signature() {
        let secret_key = [0x42u8; 32];
        let public_key = derive_public_key(&secret_key);
        let message = b"hello world";
        let wrong_message = b"wrong message";

        let signature = sign_message(&secret_key, message);
        let result = verify_signature(&public_key, wrong_message, &signature);

        assert_eq!(result, VerifyResult::Invalid);
    }

    #[test]
    fn test_deterministic() {
        let secret_key = [0x42u8; 32];
        let message = b"test message";

        let sig1 = sign_message(&secret_key, message);
        let sig2 = sign_message(&secret_key, message);

        assert_eq!(sig1, sig2);
    }
}
