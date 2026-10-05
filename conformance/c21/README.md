# Flow runtime ownership conformance v1

Track12 fixture family, consumed by Track03 native Rust tests and Track21 adapters.
`flow-runtime-identity-v1.tsv` supplies the actual bound work states and expected
errors for the foreign-terminal-work regression. The native test executes every
row using Flow resource grants and preemption; it also tests moved/cloned identity
and failed-first-admission atomicity. No raw identity or pointer is exported.

Run `cargo test --locked -p kairo-ecs-des --test fidelity_lineage_v1` under the
canonical toolchain. Run `cargo check --locked -p kairo-ecs-des --lib` under the
default engine MSRV. These local checks do not establish hosted CI or release.

This is a separate native fixture family rather than a new ready ID in the
bootstrap shared runner: that runner explicitly permits a fixed set of scheduler,
RNG and language-binding fixtures. In-process Rust ownership has no corresponding
portable or host-language wire representation. Adding a bootstrap ready ID would
misrepresent coverage. `manifest.json` documents its own consumer and limitations.
