# Bounded maturity workpack

The [task catalogue](maturity-tasks.json) refines Tracks 109–115 and S01–S17.
It describes implementation scope, dependencies and exact check arguments; it
is **not a second status ledger, autonomous executor or qualification claim**.
Track plans/registry remain authoritative. All new implementation leaves start
pending unless their actual accepted result is recorded in the parent track.

Use the repository's selected Python (or an explicitly qualified compatible
tooling environment) from the repository root:

```bash
python scripts/maturity_tasks.py check
python scripts/maturity_tasks.py list
python scripts/maturity_tasks.py show K02.arrival
```

The checker rejects missing context, oversized/directory write scopes, invalid
paths, nonexistent Cargo packages, undeclared future test targets, missing
dependencies/cycles, missing declared benchmark/simple test-gate features and lost
scientific obligations. General Rust cfg/runtime availability still requires the
actual command and a nonzero executed test count. It does not run the listed
commands, judge a mathematical oracle, select a model or mark tasks complete.

## One task, one behaviour

Each leaf names up to five source/test/output files, up to eight context files,
at most three check commands, a semantic acceptance condition and a stop rule.
Check commands use argument arrays and an explicit working directory. A future
test target must be a declared output; its command should fail until that target
is created, and it must execute nonzero tests. Design/measurement tasks produce
a bounded decision/report, not a pretend runtime test pass.

Aim for 30–90 minutes per settled local-model leaf. If the context or behaviour is
larger, split it before dispatch. Read only the named symbols/sections needed;
do not feed the complete repository history to a small model. Use
`scripts/agent_session.py context` for bounded existing source and the existing
claim/check/release protocol for writes. The catalogue's prospective write scope
does not acquire a lease or authorize unrelated changes.

`local` means suitable for supervised execution after the interface/oracle is
settled. It does not certify any model's ability. `specialist` covers unresolved
architecture, numerical method, replay or empirical interpretation. Those tasks
must emit the next bounded implementation leaves; no worker should implement a
whole new equilibrium solver, SBI method or GPU engine from a vague checkbox.

## Minimal evidence and review

Return the diff/commit, command/cwd/result, number of tests actually executed and
one unresolved-issues note. CI logs supply the routine evidence. Record seed,
input/model/version identity when stochastic/numerical work needs it. Keep large
benchmark/raw research artefacts only when they answer the stated question.
No separate packet approval, risk dossier or review panel for every ordinary
leaf. Use one review for a coherent batch; specialist review remains at public
contract, algorithm/replay and release milestones.

Red tests should distinguish the desired behaviour from the known defect. Do
not commit an unexplained failing required gate: pair the regression with its
fix in the same integration slice, or retain a clearly identified reproducer
outside the accepted test lane until its paired implementation is ready.
Missing hardware/data/toolchains are unavailable results, not skips counted as
scientific evidence. A package-wide test command is deliberately used where
an exact test filter could silently select zero tests.

## Dispatch order

Start independent K01 consumer-contract work, K02 movement contract work and
scientific completion designs. A design can use a source-derived counterexample
without waiting for the future corrected algorithm. Settle contracts before their dependent
leaves. K03 measures the serial baseline and overhead before any worker-pool
optimization. K04 native adapters and K05 packaging follow their actual provider
capability; do not claim an installed consumer from printed imports.

S01–S04 belong ultimately to the game library; their source-side regression
records protect the extraction and are not permanent duplicated solvers. Every
scientific ID has a distinguishing first task and retains the fuller acceptance
conditions in [scientific completion](../scientific-completion.md). The current
catalogue is the next executable tranche, not a false claim to have decomposed
every possible frontier experiment. A specialist must add bounded follow-on
leaves for unresolved advanced methods before implementation starts.

Consumer-owned tracks in CareOps, Innovate, UOGTO, the game incubator and Voiage
consume these interfaces. Reuse CareOps' existing `tools/tasks.py`/packet system
there; do not introduce a second dispatcher. Apply the same one-behaviour,
small-diff and milestone-review rules in every repository.
