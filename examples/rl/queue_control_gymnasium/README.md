# Queue Control Gymnasium — unavailable

This example is reserved for a Rust-owned queue-control environment. The former Python host environment is retired immediately under [the native-only policy](../../../conductor/native-only-policy.md). Constructing `KairoGymEnv` raises `NativeEnvironmentUnavailable`; installing Gymnasium does not enable an alternative simulation core.

[Track 109](../../../conductor/tracks/109-rust-only-runtime/plan.md) owns native execution and [Track 110](../../../conductor/tracks/110-scientific-method-completion/plan.md) owns real environment semantics and qualification. Publication requires a versioned model/scenario, Rust action and transition contracts, native reset/step/close plus Gymnasium checker success, replay tests, reference observations/rewards and a reproducible training example. No runnable RL capability or performance is currently claimed.
