//! Batched signature verification demo.
//!
//! Demonstrates verifying multiple signatures using ZeroOS.
//! Each signature verification can be handled by a separate thread.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use batched_signatures::{
    batch_verify, derive_public_key, sign_message, verify_signature,
    PublicKey, Signature, VerifyResult,
};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Number of signatures to verify in the batch
const BATCH_SIZE: usize = 8;

/// Generate test keypairs deterministically
fn generate_test_keypair(seed: u8) -> ([u8; 32], PublicKey) {
    let mut secret = [0u8; 32];
    for i in 0..32 {
        secret[i] = seed.wrapping_add(i as u8).wrapping_mul(17);
    }
    let public = derive_public_key(&secret);
    (secret, public)
}

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[batched-signatures] Starting signature verification demo");

    // Generate test data
    let mut secret_keys = [[0u8; 32]; BATCH_SIZE];
    let mut public_keys = [[0u8; 32]; BATCH_SIZE];
    let mut signatures = [[0u8; 64]; BATCH_SIZE];
    let mut results = [VerifyResult::Invalid; BATCH_SIZE];

    // Messages to sign/verify
    let messages: [&[u8]; BATCH_SIZE] = [
        b"transaction_0_transfer_100",
        b"transaction_1_approve_token",
        b"transaction_2_stake_amount",
        b"transaction_3_withdraw_eth",
        b"transaction_4_swap_tokens",
        b"transaction_5_add_liquidity",
        b"transaction_6_vote_proposal",
        b"transaction_7_claim_reward",
    ];

    // Generate keypairs and sign messages
    debug::writeln!("[batched-signatures] Generating {} signatures", BATCH_SIZE);
    for i in 0..BATCH_SIZE {
        let (secret, public) = generate_test_keypair(i as u8);
        secret_keys[i] = secret;
        public_keys[i] = public;
        signatures[i] = sign_message(&secret, messages[i]);
    }

    // Batch verify all signatures
    debug::writeln!("[batched-signatures] Verifying signatures...");
    batch_verify(&public_keys, &messages, &signatures, &mut results);

    // Report results
    let mut valid_count = 0;
    for (i, result) in results.iter().enumerate() {
        let status = match result {
            VerifyResult::Valid => {
                valid_count += 1;
                "VALID"
            }
            VerifyResult::Invalid => "INVALID",
        };
        println!("sig[{}] = {}", i, status);
    }

    println!("Verified {}/{} signatures as valid", valid_count, BATCH_SIZE);

    // Test with an invalid signature (wrong message)
    debug::writeln!("[batched-signatures] Testing invalid signature detection...");
    let invalid_result = verify_signature(&public_keys[0], b"wrong_message", &signatures[0]);
    if invalid_result == VerifyResult::Invalid {
        println!("Invalid signature correctly rejected");
    } else {
        println!("ERROR: Invalid signature was accepted!");
    }

    debug::writeln!("[batched-signatures] Demo complete!");
    platform::exit(0)
}
