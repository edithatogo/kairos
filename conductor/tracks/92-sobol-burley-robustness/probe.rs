// The driver extracts the generator from the immutable Kairos source.
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/generator.rs"));
fn main() {
    let mut baseline = SobolSequence::new(2);
    for i in 0..4096 {
        let native = baseline.next_point();
        let pair = |seed| {
            [
                sobol_burley::sample(i, 0, seed),
                sobol_burley::sample(i, 1, seed),
            ]
        };
        let a = pair(92);
        assert_eq!(a, pair(92));
        println!(
            "{i},{},{},{},{},{},{}",
            native[0],
            native[1],
            f64::from(a[0]),
            f64::from(a[1]),
            f64::from(pair(93)[0]),
            f64::from(pair(93)[1])
        );
    }
    assert!(sobol_burley::sample(65535, sobol_burley::NUM_DIMENSIONS - 1, 92) < 1.0);
    assert!(
        std::panic::catch_unwind(|| sobol_burley::sample(0, sobol_burley::NUM_DIMENSIONS, 92))
            .is_err()
    );
    if cfg!(debug_assertions) {
        assert!(std::panic::catch_unwind(|| sobol_burley::sample(65536, 0, 92)).is_err());
    }
    baseline.count = (1_u64 << 32) - 1;
    assert_eq!(baseline.next_point()[0], 2_f64.powi(-32));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| baseline.next_point())).is_err()
    );
}
