# S10 Native vector environments

Replace the shape-only environment with a Rust-owned finite queue model. Seed controls arrivals. Each action controls service, the observation contains the remaining demand, and the environment distinguishes completed work from a tick limit. Compare equal-seed scalar and batched trajectories, including reinitialization, hard termination and atomically rejected invalid actions.
The experimental `rl-queue` CLI feature exposes `queue-env-v1` over bounded newline-delimited JSON on child-process stdio. One process owns one queue environment. The caller supplies the native executable path, agent count, tick limit, reset seed and actions. The adapter returns the Rust observation and transition. Explicit close ends the child and invalid data leaves state intact. This route leaves the beta C ABI untouched.

Qualify the installed optional `kairo-gym` package with Gymnasium's environment checker and PettingZoo's parallel checker, using a compiled CLI. Exercise replay, host input errors, value ownership and process cleanup with the same live binding.

The qualified host route uses POSIX pipe readiness on Linux and macOS. A five-second complete-reply deadline and schema validation protect the handshake, reset and step. Invalid transport replies reap the child. Seed validation precedes changes to Gymnasium's host RNG.

The checker lock uses Gymnasium 1.4.0 and PettingZoo 1.24.3 on Python 3.15.2. Pip rejected the published PettingZoo 1.27.0 wheel because its metadata requires Python below 3.15. The newest eligible release passed the Linux-style public-index hash-lock install on macOS, followed by the installed checker cases. [PettingZoo 1.27.0 metadata](https://pypi.org/project/pettingzoo/1.27.0/) · [PettingZoo 1.24.3 metadata](https://pypi.org/project/pettingzoo/1.24.3/)
