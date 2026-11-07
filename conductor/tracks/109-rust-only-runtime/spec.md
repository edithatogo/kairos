# Track 109 specification: Rust-only runtime and native binding enforcement

## Overview and approved outcome

Retire every executable non-Rust simulation core immediately and qualify supported native/Wasm adapters without silent fallbacks.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../native-only-policy.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: bindings/, packaging/, python/kairo_gym/. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 02, 12, 25, 63 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Retirement: Every old host-engine constructor fails before state/time mutation, including with a fallback environment variable set.
- Native execution: Build the Rust library/Wasm artefact; installed test schedules/cancels/runs and verifies native event counts and error paths; missing artefact fails the native lane.
- Lifetime: Double-close, use-after-close, invalid handles, ABI mismatch, buffer ownership and overflow cases reject.
- Replay: Actual native stream generation and event traces match versioned fixtures; reading expected-vector length is insufficient.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

Breaking callers may currently depend on host traces that ABI1 cannot return; expose only real native operations and document the exact replacement. New native loading expands FFI/lifetime review obligations.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).

