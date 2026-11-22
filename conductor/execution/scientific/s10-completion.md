# S10 Native vector environments

Replace the shape-only environment with a Rust-owned finite queue model. Seed controls arrivals. Each action controls service, the observation contains the remaining demand, and the environment distinguishes completed work from a tick limit. Compare equal-seed scalar and batched trajectories, including reinitialization, hard termination and atomically rejected invalid actions.
The experimental `rl-queue` CLI feature exposes `queue-env-v1` over bounded newline-delimited JSON on child-process stdio. One process owns one queue environment. The caller supplies the native executable path, agent count, tick limit, reset seed and actions. The adapter returns the Rust observation and transition. Explicit close ends the child and invalid data leaves state intact. This route leaves the beta C ABI untouched.

Qualify the installed optional `kairo-gym` package with Gymnasium's environment checker and PettingZoo's parallel checker, using a compiled CLI. Exercise replay, host input errors, value ownership and process cleanup with the same live binding.
