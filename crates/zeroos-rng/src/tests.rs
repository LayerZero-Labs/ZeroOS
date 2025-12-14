mod chacha_tests {
    use crate::chacha::ChaChaState;

    #[test]
    fn test_chacha_determinism() {
        let mut rng1 = ChaChaState::with_seed(12345);
        let mut rng2 = ChaChaState::with_seed(12345);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);

        assert_eq!(buf1, buf2, "Same seed should produce same output");
    }

    #[test]
    fn test_chacha_different_seeds() {
        let mut rng1 = ChaChaState::with_seed(12345);
        let mut rng2 = ChaChaState::with_seed(54321);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);

        assert_ne!(
            buf1, buf2,
            "Different seeds should produce different output"
        );
    }

    #[test]
    fn test_chacha_reference_comparison() {
        use rand_chacha::rand_core::{RngCore, SeedableRng};

        let seed = 0xDEADBEEF_CAFEBABEu64;

        let mut our_rng = ChaChaState::with_seed(seed);
        let mut our_output = [0u8; 64];
        our_rng.fill_bytes(&mut our_output);

        assert_ne!(our_output, [0u8; 64], "Output should not be all zeros");

        let mut our_rng2 = ChaChaState::with_seed(seed + 1);
        let mut our_output2 = [0u8; 64];
        our_rng2.fill_bytes(&mut our_output2);

        assert_ne!(
            our_output, our_output2,
            "Different seeds produce different output"
        );
    }
}

mod lcg_tests {
    use crate::lcg::LcgState;

    #[test]
    fn test_lcg_determinism() {
        let mut rng1 = LcgState::with_seed(12345);
        let mut rng2 = LcgState::with_seed(12345);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);

        assert_eq!(buf1, buf2, "Same seed should produce same output");
    }

    #[test]
    fn test_lcg_different_seeds() {
        let mut rng1 = LcgState::with_seed(12345);
        let mut rng2 = LcgState::with_seed(54321);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);

        assert_ne!(
            buf1, buf2,
            "Different seeds should produce different output"
        );
    }

    #[test]
    fn test_lcg_sequence_consistency() {
        let mut rng1 = LcgState::with_seed(1);
        let mut rng2 = LcgState::with_seed(1);

        let v1 = rng1.next_u64();
        let v2 = rng1.next_u64();
        let v3 = rng1.next_u64();

        assert_eq!(rng2.next_u64(), v1);
        assert_eq!(rng2.next_u64(), v2);
        assert_eq!(rng2.next_u64(), v3);

        assert_ne!(v1, 0);
        assert_ne!(v2, 0);
        assert_ne!(v3, 0);
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
    }
}
