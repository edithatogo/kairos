# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

Value and behavioural gates remain unresolved. Preserve published contracts and explicit owner boundaries; see the dependency-specific specification.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Pending: `$conductor-review`, accepted fixes, rejected findings, command/cwd/commit SHA/toolchain/seed/input hashes, exit codes/artifacts, pushed ref or blocker and next-phase decision. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

## Phase 0 decision — 1 November 2026

**Defer ode_solvers.** Prerequisite Track 78 merged in [PR #317](https://github.com/edithatogo/kairos/pull/317).
At main `2137475571423f2efbec4ffdddcb52da3136a619`, the Rust call-site
inventory found one concrete continuous replicator path:
`ReplicatorDynamics::run_trajectory` calls `step_continuous_rk4`. The DES
trajectory runner is a separate API. No accepted generic fixed/adaptive
integration consumer or owner-approved trajectory migration was found. The
game-theory method receives already evaluated fitness and performs one derivative
update followed by nonnegative projection and normalization. It has no four
RK stages. The [Track 65 correctness task](../65-vectorized-marl-vecenv-advanced-game-solvers/plan.md)
owns its name, numerical contract and any versioned correction. Track 102
does not require Track 65 to wait for a solver-library decision.

### Numerical review

A Rust caller linked the actual locked game-theory library (Rust/Cargo 1.99.1,
Darwin arm64). Start from [0.25,0.75], fixed fitness [1,0], integrate to t=1.
The first fraction obeys dx/dt=x(1−x), hence x(t)=1/(1+3 exp(−t)). Every
refined step stayed nonnegative and summed to one within 1e-14.

| Steps | Absolute error at t=1 | Previous error / current |
|---:|---:|---:|
| 8 | 0.004536182918 | — |
| 16 | 0.002246699108 | 2.019043 |
| 32 | 0.001117592975 | 2.010302 |
| 64 | 0.000557305770 | 2.005350 |
| 128 | 0.000278273729 | 2.002725 |

A single step of length 8 yielded [1,0], while x(8)=0.998994623915.
Review added this control because checking only positivity/conservation would
mistake clipping for accuracy. In a second fixture fitness switches to [0,1]
at t=0.5: two equal half-intervals exactly cancel in log-odds. With 64 steps
per half the Kairos final fraction was 0.249707779377 (exact 0.25). An independent
SciPy 1.18.1 DOP853 reference on repo-local Python 3.15.2, rtol=1e-12 and
atol=1e-14, agreed with the analytical solutions at t=1 and 8 within 1e-11
and returned 0.2500000000000033 across the split. No RNG input is used.
These are baseline oracles, not performance or accuracy evidence for ode_solvers.

The native caller used `cargo build --locked --offline -p kairo-ecs-game-theory
--lib` and rustc with the resulting rlib. Its source and raw outputs are retained
in the local receipt directory below. For each n in [8,16,32,64,128], call
`step_continuous_rk4(&state, &[1.0, 0.0], 1.0/n)` n times from the same
initial condition; compute the error against the formula above.

Crates.io identified ode_solvers 0.6.2 as the latest stable non-yanked release
(Apache-2.0; no declared rust_version in release metadata). Phase 0 stopped
before installing the candidate. No crate features, runtime targets, solver
tolerances, security or performance acceptance are asserted. Reopen when the
owner supplies an actual derivative/model API, required solver class,
event/restart semantics, numerical tolerances, and a versioning plan for
trajectories. CareOps Sim's 102-ode-solvers-qualification row remains unresolved
with pending impact review; no clinical equations or pins changed.

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns this track directory in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos on codex/track102-ode-decision.
Native scope: local numerical checks and read-only registry/consumer inspection.
The integrator retains shared registry and phase acceptance. Next operation
is protected integration.

Receipt SHA-256 values (the Rust caller and output are reproduced below):

- `probe.rs` — e8cdd6c6f7225933b871ee38f680cb5c3d8e2fafd09bb7cb086dbc6b03cd2d88
- `probe.log` — 8add28ea1862f1a385a2cc8809dfb2b35d0422cdac21edd880b097460b4f24af
- `reference.log` — ca1f240b9fbaa9f76ccb247cd0c3202707ec6f6043fed203815909c7a7c068e2


### Portable reproduction

Save the following caller as `probe.rs` in a scratch directory. From an isolated checkout of the baseline revision, use the repository toolchain to build and run it:

```sh
cargo build --locked -p kairo-ecs-game-theory --lib --message-format=json > build.json
lib=$(jq -r 'select(.reason=="compiler-artifact" and .target.name=="kairo_ecs_game_theory") | .filenames[] | select(endswith(".rlib"))' build.json)
rustc --edition 2021 --extern "kairo_ecs_game_theory=$lib" -L "dependency=$(dirname "$lib")/deps" /path/to/probe.rs -o /path/to/probe
/path/to/probe
```


```rust
use kairo_ecs_game_theory::normal_form::ReplicatorDynamics as R;
fn main() {
    // Fitness [1,0] gives dx/dt = x(1-x); x(0)=1/4.
    let exact = 1.0 / (1.0 + 3.0 * (-1.0_f64).exp());
    let mut prev = None;
    for n in [8,16,32,64,128] {
        let mut state = vec![0.25, 0.75];
        for _ in 0..n {
            state = R::step_continuous_rk4(&state,&[1.0,0.0],1.0/f64::from(n));
            assert!(state.iter().all(|&v| (0.0..=1.0).contains(&v)));
            assert!((state.iter().sum::<f64>()-1.0).abs()<1e-14);
        }
        let err=(state[0]-exact).abs();
        if let Some(previous) = prev {
            let ratio: f64 = previous/err;
            assert!((1.8..2.2).contains(&ratio), "observed order changed");
            println!("{n} {err:.12} {ratio:.6}");
        } else { println!("{n} {err:.12} -"); }
        prev=Some(err);
    }
    let projected=R::step_continuous_rk4(&[0.25,0.75], &[1.0,0.0], 8.0);
    assert_eq!(projected,[1.0,0.0]);
    println!("oversized step {projected:?}; exact {:.12}",1.0/(1.0+3.0*(-8.0_f64).exp()));
    let mut state=vec![0.25,0.75];
    for fitness in [[1.0,0.0],[0.0,1.0]] {
        for _ in 0..64 {state=R::step_continuous_rk4(&state,&fitness,1.0/128.0);}
    }
    println!("fitness switches at t=0.5, final {:.12}, exact 0.25",state[0]);
    assert!((state[0]-0.25).abs()<0.001);
}
```

Expected output:

```text
8 0.004536182918 -
16 0.002246699108 2.019043
32 0.001117592975 2.010302
64 0.000557305770 2.005350
128 0.000278273729 2.002725
oversized step [1.0, 0.0]; exact 0.998994623915
fitness switches at t=0.5, final 0.249707779377, exact 0.25
```
