# Stable scheduler property execution

The 20 November 2026 portfolio rollout adds executable stable assurance to the
existing scheduled/manual fuzz status workflow, and triggers the same suite on
pull requests that change its scheduler inputs. Track 96 already provides the
generators and independent oracles; no second property framework is introduced.

The `Scheduler properties (Rust 1.99.1)` job builds and executes both existing
Track 96 integration targets with locked dependencies. Their seven tests cover
seed replay, shrinking, negative controls, total ordering, conservation,
cancellation and generational identities. Generators retain their checked-in
fixed seeds and case counts. Compilation and execution have separate 600-second
and 120-second process budgets, inside a 15-minute job limit.

The workflow retains the explicit `Fuzzing UNVERIFIED (cargo-fuzz not run)` job.
These stable properties do not establish coverage-guided libFuzzer execution,
AddressSanitizer qualification, or completion of the nightly release obligation.
No compiler policy, root dependency, accepted Track 96 status, scheduler source,
or frozen integration PR is changed.

## Reproduction and review

```bash
cargo +1.99.1 test --locked -p kairo-ecs-core \
  --test track96_phase0_proptest_baseline \
  --test track96_phase1_proptest_invariants -- --test-threads=1 --nocapture
```

Workflow artifacts retain the checked-out commit, compiler identity, build/test
logs and any generated property regression seed files for fourteen days. A
failed command remains a failed job; no retries or allowed-failure step is added.
PR code executes with read-only contents permission and no persisted credentials.

Candidate validation is recorded in the PR. This note records implementation
scope and commands; it is not a native-test, hosted-run, or release acceptance.
