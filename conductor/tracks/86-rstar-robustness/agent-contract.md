# Ownership contract

abm-api-agent owns the bounded implementation target: crates/kairo-ecs-abm/src/lib.rs. Coordinator owns registry decisions; manifest/lockfile/CI/pin owners must hand off before shared changes. No cross-repository writes. Existing claims, including expired reservations, are not cleared by this plan.

Use isolated worktree, advisory claim, bounded hashed context and precommit scope check; immutable inputs exclude intended output files. One commit per claim, then release. Value-gate decision must be explicit before Phase 1. Follow [policy](../../dependency-value-policy.md).
