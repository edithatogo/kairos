# Risk Register: Track 30 Toolchain & Version Support Matrix

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

Last updated: 2026-05-07.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger | Current status |
|---|---:|---:|---:|---|---|---|---|
| A binding track ships with an undocumented minimum version | 2 | 4 | 8 | Matrix now records each Track 06-11 manifest floor or explicit gap; `toolchain-check.yml` triggers on manifest changes. | toolchain-agent | Any binding enters beta without min-version declaration in `conductor/toolchain-matrix.md`. | Mitigated for current manifests; TypeScript still lacks `engines.node`, so Track 09 should add it when source scope opens. |
| CI runner drops a version without updating the matrix | 2 | 4 | 8 | `toolchain-check.yml` sets up every declared CI-covered selector and calls `validate-toolchain-matrix.ps1 -CheckInstalled`. | toolchain-agent | A setup action cannot install a declared selector or the installed version prefix differs. | Mitigated for Track 30 workflow; full confidence requires a live GitHub Actions run. |
| A language ecosystem releases a new major version faster than the update cycle | 3 | 3 | 9 | Matrix now requires refresh within one KairoECS release cycle after upstream GA. | toolchain-agent | New major version unsupported 2+ release cycles after release. | Open monitoring risk; upstream checks remain manual. |
| Version-drop policy is ignored by binding tracks | 2 | 4 | 8 | Matrix includes required notice sequence, removal criteria, waiver rule, and proposed drops table; gate definition added. | release-agent | Version dropped without ADR/waiver or policy-compliant notice. | Mitigated for future policy checks; historical diff comparison remains future validator work. |
| OS/arch support matrix is incomplete | 3 | 3 | 9 | Matrix labels Ubuntu x86_64 as `CI-covered` and other platforms as `best-effort` until Track 13 provisions runners. | toolchain-agent | Required-platform cell remains `best-effort` at RC. | Open release-hardening risk; Track 13 runner expansion needed before RC. |
| Toolchain matrix diverges from CI runner reality | 2 | 4 | 8 | Workflow triggers on matrix, gate, workflow, rust/mise/Cargo, and Track 06-11 manifest changes. | ci-agent | `toolchain-check.yml` fails. | Mitigated once GitHub Actions confirms the new workflow. |
| Multiple binding tracks declare conflicting minimum versions for the same language | 2 | 3 | 6 | Matrix is documented as the single source of truth and records manifest evidence separately from CI support floors. | toolchain-agent | Two binding manifests raise floors outside Track 30 policy. | Mitigated by manifest-triggered validation and handoff. |
| Existing package dry-run workflow still references Go 1.24 | 2 | 3 | 6 | Matrix deprecates the Go 1.24 package dry-run lane and hands off update to Track 13/15 because existing workflow edits are out of Track 30 scope. | ci-agent + release-agent | Release packaging still runs Go 1.24 at beta. | Open scoped blocker for Track 13/15, not changed in this slice. |
