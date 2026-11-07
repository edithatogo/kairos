# Strategic-information execution provenance

Date: 8 November 2026. Status: additional design/acceptance input; implementation
pending. Owners remain Track 114 K01.games/K01.port and the existing execution
workpack. This is not a new trait, ABI, scheduler or runtime capability registry.

## Preserve the independent release paths

Kairos owns Rust-native simulation time, events, state, resources, RNG and replay.
T17 in the game incubator owns generic representations and corrected solvers;
UOGTO owns semantic profiles; applications own games and valuations; Voiage owns
information-value calculations. Core simulation, bounded static game packages and
the optional dynamic adapter keep separate acceptance/release evidence.

No game dependency or strategic-VOI requirement is added to the Kairos core release.
Static game solving has no Kairos dependency, even transitively. Voiage numerical
job cancellation/checkpoints are not the simulation clock. Preserve inherited
S01–S04 source regressions, callers and retirement/transfer records: moving code
or supplying a trace does not discharge their mathematical obligations.

## Refine existing K01.port and T17.13–16 acceptance

- [ ] Map full game specification/instance, player/policy, session and trace IDs to
  qualified native generational handles. Reject stale generations, reused session
  identities, unknown actions and lossy namespace conversions.
- [ ] Emit causal events only for a selected policy and declared chance execution.
  Solver visits to hypothetical/unchosen branches remain diagnostics, not events,
  resource consumption or observed patient/service outcomes.
- [ ] Bind permitted observation/information-set IDs and observation revision to
  policy calls. Full mathematical solver state must not reach acting policies as
  a convenience payload. Test two hidden states with identical lawful observations.
- [ ] Keep analyst uncertainty draws, policy decisions, simulation replications and
  solver runs distinct. Export explicit common-world coupling where comparisons use
  paired outcomes; matching seeds alone cannot certify such a coupling.
- [ ] Isolate solver RNG purposes from exogenous native event streams. Changing
  solver budget, warm start or cache use must not silently perturb the exogenous
  scenario. Record the policy/selection identity and supported random-stream contract.
- [ ] Checkpoint/resume only qualified policy, native state and RNG combinations.
  Preserve causal event order, cancellation and stale/duplicate-action handling;
  unsupported replay remains an explicit error, not a host-engine fallback.
- [ ] Export provenance and physical/resource outcomes to application-owned valuation.
  Do not compute welfare, QALYs, EVPI/EVSI or reimbursement objectives inside Kairos.

## Verification and handoff

Augment the existing execution-port tests, not a new acceptance registry: identity
round trips; hidden-state negatives; diagnostic-versus-causal trace separation;
unchanged exogenous draws under solver-budget changes; stale command/cancellation;
policy/native checkpoint equivalence; invalid trace origin and broken pairings.
Static provider tests execute without Kairos. Dynamic tests must execute real Rust
native paths and preserve code/feature/toolchain/input hashes and exact outcomes.

Bind exact symbols and tests in K01.port before code changes. Use red witnesses,
minimal changes and the selected track's focused checks, followed by required
native/replay/installed and hosted gates. Existing unsupported statuses, provider
pins, source obligations and public contracts are unchanged by this roadmap entry.
No task is marked complete and no external release or publication occurs.

## Coordinated Voiage plan

See the [shared SI01–SI04 plan and SI-V01–SI-V08 cases](https://github.com/edithatogo/voiage/blob/c414752a228e7af5d689d9d5876f8f7372c42447/conductor/tracks/scientific_ecosystem_alignment_20261029/strategic-information-integration.md).
That source commit is a planning proposal, not a dependency pin or an acceptance receipt.
