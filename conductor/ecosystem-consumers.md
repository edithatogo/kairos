# Ecosystem consumer inventory and delivery handoffs

Source inspection: 7 November 2026. This is a bounded inventory of the owner's
visible repositories and code-search results, not a claim that no other consumers
exist. Classification follows manifests and executable source, not project names
or old completed tracks. No dependency pin is changed by this review.

Game-library follow-up: 8 November 2026 (Australia/Brisbane). The first inventory
below retains its inspected source revisions. Current receiving coordination is
[game-runtime issue 100](https://github.com/edithatogo/new-drug-reimbursement-game/issues/100)
and T17, aligned with Innovate's G01–G04 programme. **Paignia is the maintainer's
chosen name and [actual repository](https://github.com/edithatogo/paignia).**
The user supplied that newly created repository after the initial bounded lookup.
Transfer the core/solve seed there; PR 101 makes the old incubator an immutable
consumer. Naming, source movement, installed acceptance and publication remain
distinct. Historical rows below retain their original pins. Canonical T17
provider work moves to Paignia; domain-specific .09/.11 remain reimbursement
application work and .10 belongs to the chosen second consumer.

| Repository / inspected revision | Observed relationship | Required consumer-owned work |
| --- | --- | --- |
| `careops-sim` / `672d3e8f78f5e502a760b9d41200fdbaf4d9d144` | Direct Rust path dependencies on Kairos types/core/RNG/DES under `libs/kairos`; actual gitlink `07410266bf54b6f73c59db2f8cbfbc5697c53734` | Reconcile consumed staged-allocation and boundary APIs; install a minimal immutable package/commit fixture; keep physical/clinical policies local; adopt neutral usage/movement through a thin adapter |
| `innovate` / `b9998addf32717438df92b23a306903e4db51620` | Seven declared Rust Git dependencies at `fae901558f07b7b717a676adbafbe2cdc78dea1c`; DES/ABM smoke examples print intended imports rather than executing them | Replace source/metadata-only smoke with actual Rust scheduling/state/RNG; preserve diffusion models; make heavy data features optional where compatible; qualify an explicit adapter without host-core fallback |
| `new-drug-reimbursement-game` / `ab4ce37a905a8cf9bb54d1ad905de4b0f34b7165` | Game crates already separate; Kairos event-export proposal and lock entry, no Kairos Cargo dependency | Incubate generic game packages, executable UOGTO profile and optional Kairos adapter; retain reimbursement economics and specific games here |
| `UOGTO` / `ac5b8e86c4a3547d008e83ba4cfcbb5f3ba95f25` | Ontology/SHACL/JSON-LD provider and Python reference/demo runner | Version a supported executable profile and mapping fixtures; preserve semantic identities; distinguish ontology tooling from a production game/simulation core |
| `voiage` / `a5d87a736fa9f0f876e44872d946b2170d2a0898` | Cross-domain VOI provider and existing uncertainty exchange; no required Kairos Rust dependency | Accept paired strategy/draw outcome or net-benefit records through optional producer adapters; reject unit/perspective/draw mismatch; keep existing non-HPC work eligible |
| `nhra_game` / `3fe4aaa08521d103ddffbedd58eaa9d7d5256a55` (follow-up inspection) | Prospective domain consumer: Python `nhra_gt` 26.0.2 declares JAX/JAXopt/PyGambit; source owns monthly state/RNG transitions and local strategic solvers. It is not an installed Rust game/Kairos consumer | Select one independently checked static game for shared Rust solver adoption; keep NHRA institutions and payoff assumptions local. Qualify any later monthly native execution separately; do not count declared dependencies or the current Python/JAX engine as native acceptance |
| `closer-to-whom` | Capacity-envelope Arrow/Parquet proposal and fixture; local `implied_capacity` computes implied workload | Qualify schema/version/units and distinguish implied requirements from observed or simulated feasible capacity; no scheduler required for static aggregation |
| `gtpcnz` | A page labelled Kairos playback calls local `ABMSimulation`; the inspected page does not establish native Kairos dispatch | Audit the execution path before using a Kairos-backed claim; either consume real Rust output or label the local model/playback accurately |
| `foi-process` | Proposed synthetic ProcessEvent/OCEL export, not evidence of an installed Kairos engine | Define a fictional queue model and versioned trace export; keep FOI rules/privacy classification in the domain and process-mining semantics in their provider |

The older CareOps record retaining `44261eb` describes a historical acceptance;
it is **not** the current gitlink. Inspect the actual manifest and gitlink before
preparing an upgrade. A newer Kairos main is not automatically a compatible
replacement for a consumer's selected branch.

The 8 November follow-up inspected CareOps main
`a44a8c9f1461839c5306dbf44a1fa9a3b1981c50`, still pinning Kairos
`07410266bf54b6f73c59db2f8cbfbc5697c53734`. It confirms an existing
`careops-core` package and an actual nonclinical logistics test consumer for its
experiment module. Its configurable-pathway loop/custom RNG and sequential
ED/ward/community composition require the [bounded owner follow-ups](design/careops-composition-plan-20261020.md#paignia-and-careops-boundary-review).
These source findings do not qualify a newer provider pin or native execution.
CareOps issue [205](https://github.com/edithatogo/careops-sim/issues/205) maps the
optional Paignia profile to R76.5; it is not mandatory for T17's first application
consumer or for CareOps R76.6/R77 runtime acceptance.

## Repository-local workstreams

Track 114 coordinates interfaces and handoffs; each repository owns its adapter
and tests. The downstream workstreams are `kairos_domain_contracts_20261107`
(CareOps), `kairos_runtime_boundary_20261107` (Innovate), the game-runtime
extraction workstream in the reimbursement game, executable semantic profiles
in UOGTO, and simulation-outcome/VOI contracts in Voiage. Their registration and
merge state must be read in the target repository, not inferred from this list.

Innovate PR 570 already proposes a frontier/Rust-core programme. The Kairos
consumer track complements that work; it does not create a competing diffusion
roadmap or silently take over its branch. CareOps' open composition and
dependency PRs remain separate from this interface plan.

For the three data/proposal consumers, keep small handoff tasks here until an
actual provider contract is selected: C-DATA-1 capacity units and schema fixture;
C-DATA-2 playback native-dispatch/label reconciliation; C-DATA-3 synthetic OCEL
export. Their implementation belongs in the consumer, not the Kairos kernel.
Do not force a static data calculation to depend on a simulation runtime.

For prospective NHRA consumption, retain three bounded handoffs here until its
consumer-owned track and provider capability are selected: C-GAME-1 freezes one
two-action game, assumptions, legal actions and independent expected result from
`src/nhra_gt/subgames/nash.py` / `solvers_jax.py`; C-GAME-2 adds one actual installed
Rust static solver case, versioned semantic-ID mapping and explicit unsupported
result; C-GAME-3 separately qualifies one monthly native transition from
`src/nhra_gt/engine.py`, including clock/RNG/replay identity. That application
must select exact files/commands before implementation. Its local algorithms
are candidate comparisons, not accepted numerical oracles merely because they
already exist. No NHRA migration is claimed by this inventory.

The [extraction map](contracts/game-extraction-map.md) and
[execution port](contracts/game-execution-port.md) govern all game consumers.
CareOps/Innovate core-runtime receipts and UOGTO static semantic receipts must
not wait for optional dynamic-game receipts. Preserve optional tasks with their
own scope; actual provider pin changes follow installed acceptance. The game
library's solver diagnostic trace must not be mistaken for a causal event trace,
and Voiage cannot infer producer RNG coupling or valid valuation from paired IDs
alone.

## Contract ownership and dependency order

1. Kairos owns neutral engine IDs/time/events/usage schemas and native error,
   replay and lifetime behaviour. Minimal upstream tests use public fictional
   fixtures and must not require private consumer access.
2. UOGTO owns semantic identifiers and executable-profile mappings. The game
   library owns solver inputs/results, residuals, tolerances and capabilities.
3. Domain packages own physical, clinical, diffusion and economic assumptions.
   A reusable HEOR module is proposed above the usage/outcome boundary; the
   package name and repository are intentionally not fixed prematurely.
4. Voiage owns the uncertainty/result contract for its methods. Producers own
   pairing, valuation and provenance before handing outcomes over.
5. A consumer upgrades its immutable provider pin in its own change after its
   real installed fixture passes. Record provider and consumer revisions once.

For each interface use one tiny positive fixture and meaningful negative cases:
unsupported capability, stale ID/schema, wrong time/unit, duplicate/conflicting
event, broken draw pairing or invalid lifetime. Reuse that fixture across
packages. A larger evidence dossier is warranted only by a concrete scientific,
compatibility, privacy or release question.

## Inspected source links

- [CareOps manifest](https://github.com/edithatogo/careops-sim/blob/672d3e8f78f5e502a760b9d41200fdbaf4d9d144/Cargo.toml)
- [Innovate manifest](https://github.com/edithatogo/innovate/blob/b9998addf32717438df92b23a306903e4db51620/bindings/rust/Cargo.toml) and [DES example](https://github.com/edithatogo/innovate/blob/b9998addf32717438df92b23a306903e4db51620/bindings/rust/examples/kairos_des_smoke.rs)
- [Existing game boundary ADR](https://github.com/edithatogo/new-drug-reimbursement-game/blob/ab4ce37a905a8cf9bb54d1ad905de4b0f34b7165/docs/architecture/adr/0002-uogto-above-kairos.md)
- [UOGTO execution bindings](https://github.com/edithatogo/UOGTO/blob/ac5b8e86c4a3547d008e83ba4cfcbb5f3ba95f25/ontologies/extensions/kg-execution-bindings.ttl)
- [Voiage Cargo workspace](https://github.com/edithatogo/voiage/blob/a5d87a736fa9f0f876e44872d946b2170d2a0898/Cargo.toml)
- [NHRA manifest](https://github.com/edithatogo/nhra_game/blob/3fe4aaa08521d103ddffbedd58eaa9d7d5256a55/pyproject.toml), [monthly engine](https://github.com/edithatogo/nhra_game/blob/3fe4aaa08521d103ddffbedd58eaa9d7d5256a55/src/nhra_gt/engine.py) and [local solver surface](https://github.com/edithatogo/nhra_game/blob/3fe4aaa08521d103ddffbedd58eaa9d7d5256a55/src/nhra_gt/subgames/nash.py)
- [Capacity envelope proposal](https://github.com/edithatogo/closer-to-whom/blob/main/upstream/issues/kairos-capacity-envelope-arrow.md)
- [Playback page](https://github.com/edithatogo/gtpcnz/blob/91a6519f22f9390ba1c336e34c4305c0a66622e1/models/primarycare_model/pages/2_kairos_abm_playback.py)
- [FOI export proposal](https://github.com/edithatogo/foi-process/blob/3939d9c83eae196e5003b77ff0fe49abce2822a7/repo-exports/kairos/foi-synthetic-ocel.md)
