# Q4 ABM registration phase prerequisite v1

The shared ABM contract requires registration before the first successful work
creation anywhere in the runtime. Existing DES registration validates individual
keys and permits new keys after other work; that behavior remains available.

Add `FlowRuntime::ensure_pre_work_registration() -> Result<(), FlowError>`.
It checks the existing run/halt gate, then rejects with `InvalidWork` when the
historical context-type map is nonempty. That map is populated only after
successful task/carrier creation and remains populated after cleanup. Actor or
resource creation, descriptor registration, and rejected work creation do not
close the phase. The method is read-only and creates no additional state.

The new ABM registration helper calls this gate before the existing typed view
descriptor registration. Existing DES registration APIs, world/time ownership,
RNG derivation, callback semantics and admission behavior are unchanged.

This additive prerequisite requires failing fixtures first, both Rust toolchain
DES regressions, and then requalification of the unchanged ABM public fixtures
on the joined implementation. It does not close full Q4 or release acceptance.
