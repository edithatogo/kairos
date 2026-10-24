#![forbid(unsafe_code)]

use kairo_ecs_rng::rand_distr::{
    Bernoulli, Binomial, Exp, Exp1, Geometric, Normal, Open01, StandardNormal, Uniform, Weibull,
};
use kairo_ecs_rng::DeterministicStream;

#[test]
fn test_uniform_float_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1001);
    let dist = Uniform::new(10.0, 20.0).expect("valid range");

    let n = 50_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for _ in 0..n {
        let x: f64 = stream.sample(dist);
        assert!((10.0..20.0).contains(&x), "sample {x} outside [10.0, 20.0)");
        sum += x;
        sum_sq += x * x;
    }

    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - (mean * mean);

    // Theoretical: Mean = (10 + 20) / 2 = 15.0
    // Theoretical: Variance = (20 - 10)^2 / 12 = 100 / 12 ~ 8.3333
    assert!(
        (mean - 15.0).abs() < 0.05,
        "Uniform float mean drifted: observed {mean}, expected 15.0"
    );
    assert!(
        (variance - 8.3333).abs() < 0.15,
        "Uniform float variance drifted: observed {variance}, expected 8.3333"
    );

    // Invalid range rejection
    assert!(Uniform::new(20.0, 10.0).is_err());
    assert!(Uniform::new(10.0, 10.0).is_err());
}

#[test]
fn test_uniform_integer_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1002);
    let dist = Uniform::new(1i32, 7i32).expect("valid range"); // 1..7 (fair 6-sided die)

    let n = 60_000;
    let mut counts = [0usize; 7];

    for _ in 0..n {
        let roll = stream.sample(dist);
        assert!((1..=6).contains(&roll));
        counts[roll as usize] += 1;
    }

    // Each face should have prob 1/6 ~ 0.166667
    for (face, &count) in counts.iter().enumerate().skip(1) {
        let p = count as f64 / n as f64;
        assert!(
            (p - 1.0 / 6.0).abs() < 0.01,
            "Face {face} probability drifted: observed {p}, expected 0.1667"
        );
    }

    // Invalid integer range rejection
    assert!(Uniform::new(7i32, 1i32).is_err());
    assert!(Uniform::new(5i32, 5i32).is_err());
}

#[test]
fn test_open01_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1003);

    let n = 50_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for _ in 0..n {
        let u: f64 = stream.sample(Open01);
        assert!(
            u > 0.0 && u < 1.0,
            "Open01 must be strictly in (0, 1), got {u}"
        );
        sum += u;
        sum_sq += u * u;
    }

    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - (mean * mean);

    // Theoretical: Mean = 0.5, Variance = 1/12 ~ 0.083333
    assert!(
        (mean - 0.5).abs() < 0.01,
        "Open01 mean drifted: observed {mean}, expected 0.5"
    );
    assert!(
        (variance - 0.083333).abs() < 0.01,
        "Open01 variance drifted: observed {variance}, expected 0.083333"
    );
}

#[test]
fn test_exponential_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1004);
    let lambda = 2.0;
    let dist = Exp::new(lambda).expect("valid rate");

    let n = 50_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    let mut cdf_below_median = 0usize;

    for _ in 0..n {
        let x: f64 = stream.sample(dist);
        assert!(x >= 0.0, "Exponential draw must be non-negative");
        sum += x;
        sum_sq += x * x;
        // CDF: F(t) = 1 - exp(-lambda * t). Median is ln(2) / lambda
        let median = 2.0f64.ln() / lambda;
        if x <= median {
            cdf_below_median += 1;
        }
    }

    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - (mean * mean);

    // Theoretical: Mean = 1 / lambda = 0.5
    // Theoretical: Variance = 1 / lambda^2 = 0.25
    assert!(
        (mean - 0.5).abs() < 0.015,
        "Exp mean drifted: observed {mean}, expected 0.5"
    );
    assert!(
        (variance - 0.25).abs() < 0.02,
        "Exp variance drifted: observed {variance}, expected 0.25"
    );

    let median_frac = cdf_below_median as f64 / n as f64;
    assert!(
        (median_frac - 0.5).abs() < 0.015,
        "Exp median quantile drifted: observed {median_frac}, expected 0.5"
    );

    // Exp1 standard exponential (lambda = 1.0)
    let mut sum1 = 0.0;
    for _ in 0..n {
        let x: f64 = stream.sample(Exp1);
        assert!(x >= 0.0);
        sum1 += x;
    }
    let mean1 = sum1 / n as f64;
    assert!(
        (mean1 - 1.0).abs() < 0.02,
        "Exp1 mean drifted: observed {mean1}, expected 1.0"
    );

    // Degenerate and invalid parameters
    let exp_zero = Exp::new(0.0).expect("zero lambda is allowed as degenerate zero hazard");
    let exp_zero_sample: f64 = stream.sample(exp_zero);
    assert_eq!(
        exp_zero_sample,
        f64::INFINITY,
        "zero lambda yields infinite waiting time"
    );
    assert!(Exp::new(-1.0).is_err());
    assert!(Exp::new(f64::NAN).is_err());
}

#[test]
fn test_normal_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1005);
    let mu = 5.0;
    let sigma = 2.0;
    let dist = Normal::new(mu, sigma).expect("valid parameters");

    let n = 50_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    let mut within_one_sigma = 0usize;
    let mut within_two_sigma = 0usize;

    for _ in 0..n {
        let x: f64 = stream.sample(dist);
        sum += x;
        sum_sq += x * x;
        if (x - mu).abs() <= sigma {
            within_one_sigma += 1;
        }
        if (x - mu).abs() <= 2.0 * sigma {
            within_two_sigma += 1;
        }
    }

    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - (mean * mean);

    // Theoretical: Mean = 5.0, Variance = 4.0
    assert!(
        (mean - 5.0).abs() < 0.05,
        "Normal mean drifted: observed {mean}, expected 5.0"
    );
    assert!(
        (variance - 4.0).abs() < 0.1,
        "Normal variance drifted: observed {variance}, expected 4.0"
    );

    // 68.27% within 1 sigma, 95.45% within 2 sigma
    let frac1 = within_one_sigma as f64 / n as f64;
    let frac2 = within_two_sigma as f64 / n as f64;
    assert!(
        (frac1 - 0.6827).abs() < 0.015,
        "1-sigma fraction drifted: observed {frac1}, expected 0.6827"
    );
    assert!(
        (frac2 - 0.9545).abs() < 0.01,
        "2-sigma fraction drifted: observed {frac2}, expected 0.9545"
    );

    // Standard normal
    let mut std_sum = 0.0;
    for _ in 0..n {
        let x: f64 = stream.sample(StandardNormal);
        std_sum += x;
    }
    let std_mean = std_sum / n as f64;
    assert!(
        std_mean.abs() < 0.02,
        "StandardNormal mean drifted: observed {std_mean}, expected 0.0"
    );

    // Degenerate sigma = 0.0 yields deterministic mean point mass
    let normal_zero_sigma =
        Normal::new(42.0, 0.0).expect("sigma=0 allowed as degenerate point mass");
    assert_eq!(stream.sample(normal_zero_sigma), 42.0);

    // Non-finite standard deviation rejection (BadVariance)
    assert!(Normal::new(0.0, f64::INFINITY).is_err());
    assert!(Normal::new(0.0, f64::NAN).is_err());
}

#[test]
fn test_bernoulli_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1006);
    let p = 0.35;
    let dist = Bernoulli::new(p).expect("valid probability");

    let n = 50_000;
    let mut trues = 0usize;

    for _ in 0..n {
        if stream.sample(dist) {
            trues += 1;
        }
    }

    let observed_p = trues as f64 / n as f64;
    assert!(
        (observed_p - p).abs() < 0.01,
        "Bernoulli probability drifted: observed {observed_p}, expected {p}"
    );

    // Degenerate p = 0.0 and p = 1.0
    let dist_zero = Bernoulli::new(0.0).expect("valid");
    let dist_one = Bernoulli::new(1.0).expect("valid");
    for _ in 0..100 {
        assert!(!stream.sample(dist_zero));
        assert!(stream.sample(dist_one));
    }

    // Invalid probability rejection
    assert!(Bernoulli::new(-0.1).is_err());
    assert!(Bernoulli::new(1.1).is_err());
    assert!(Bernoulli::new(f64::NAN).is_err());
}

#[test]
fn test_binomial_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1007);
    let n_trials = 20u64;
    let p = 0.4;
    let dist = Binomial::new(n_trials, p).expect("valid parameters");

    let samples = 30_000;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for _ in 0..samples {
        let k: u64 = stream.sample(dist);
        assert!(k <= n_trials, "Binomial draw {k} exceeds n {n_trials}");
        sum += k as f64;
        sum_sq += (k * k) as f64;
    }

    let mean = sum / samples as f64;
    let variance = (sum_sq / samples as f64) - (mean * mean);

    // Theoretical: Mean = n * p = 20 * 0.4 = 8.0
    // Theoretical: Variance = n * p * (1 - p) = 20 * 0.4 * 0.6 = 4.8
    assert!(
        (mean - 8.0).abs() < 0.05,
        "Binomial mean drifted: observed {mean}, expected 8.0"
    );
    assert!(
        (variance - 4.8).abs() < 0.1,
        "Binomial variance drifted: observed {variance}, expected 4.8"
    );

    // Degenerate cases
    let dist_zero_p = Binomial::new(10, 0.0).expect("valid");
    let dist_one_p = Binomial::new(10, 1.0).expect("valid");
    let dist_zero_n = Binomial::new(0, 0.5).expect("valid");
    assert_eq!(stream.sample(dist_zero_p), 0);
    assert_eq!(stream.sample(dist_one_p), 10);
    assert_eq!(stream.sample(dist_zero_n), 0);

    // Invalid parameters rejection
    assert!(Binomial::new(10, -0.1).is_err());
    assert!(Binomial::new(10, 1.1).is_err());
}

#[test]
fn test_geometric_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1008);
    let p = 0.25;
    let dist = Geometric::new(p).expect("valid probability");

    let n = 40_000;
    let mut sum = 0.0;

    for _ in 0..n {
        let k: u64 = stream.sample(dist);
        sum += k as f64;
    }

    let mean = sum / n as f64;
    // Geometric in rand_distr models number of failures before first success: Mean = (1 - p) / p = 0.75 / 0.25 = 3.0
    assert!(
        (mean - 3.0).abs() < 0.08,
        "Geometric mean drifted: observed {mean}, expected 3.0"
    );

    // Degenerate p = 0.0 (infinite failures / never succeeds)
    let geom_zero = Geometric::new(0.0).expect("p=0 allowed");
    assert_eq!(stream.sample(geom_zero), u64::MAX);

    // Invalid parameter rejection
    assert!(Geometric::new(1.1).is_err());
    assert!(Geometric::new(-0.5).is_err());
}

#[test]
fn test_weibull_independent_oracle() {
    let mut stream = DeterministicStream::from_run_seed(1009);
    // Shape = 1.0 corresponds to Exponential distribution with scale lambda
    let scale = 3.0;
    let shape = 1.0;
    let dist = Weibull::new(scale, shape).expect("valid parameters");

    let n = 30_000;
    let mut sum = 0.0;

    for _ in 0..n {
        let x: f64 = stream.sample(dist);
        assert!(x >= 0.0);
        sum += x;
    }

    let mean = sum / n as f64;
    // For Weibull with shape = 1, mean = scale = 3.0
    assert!(
        (mean - 3.0).abs() < 0.08,
        "Weibull (shape=1) mean drifted: observed {mean}, expected 3.0"
    );

    // Invalid parameters rejection
    assert!(Weibull::new(0.0, 1.0).is_err());
    assert!(Weibull::new(1.0, 0.0).is_err());
    assert!(Weibull::new(-1.0, 1.0).is_err());
}
