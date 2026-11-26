# Track 105 Active Python Entry-Point & Environment Inventory

Audited on `2026-11-27` against `.venv/bin/python` (`Python 3.15.2`).

## 1. Selected Runtime & Lockfiles

- **Selected Default Interpreter:** `.venv/bin/python` (`CPython 3.15.2`, verified by `scripts/python_runtime.py`)
- **Package Manifests (`pyproject.toml`, 3 total):**
  - `bindings/python/pyproject.toml`
  - `python/kairo_gym/pyproject.toml`
  - `templates/bindings/python/pyproject.toml`
- **Python Lockfiles (7 total, excluding `Cargo.lock`):**
  - `scripts/bootstrap-python-tools.lock`
  - `scripts/package-python-tools.lock`
  - `scripts/archive-python-tools.lock`
  - `python/kairo_gym/ci-requirements.lock`
  - `scripts/supply_chain/syft-darwin-verifier.lock`
  - `scripts/supply_chain/syft-linux-verifier.lock`
  - `conductor/tracks/92-sobol-burley-robustness/probe.lock`

## 2. Active CI Workflows (21 workflows, all pinned to `3.15.2`)

- `.github/workflows/agent-harness.yml`
- `.github/workflows/archive-supply-chain-main.yml`
- `.github/workflows/bench-regression.yml`
- `.github/workflows/benchmark-smoke.yml`
- `.github/workflows/changelog-policy.yml`
- `.github/workflows/ci-bindings.yml`
- `.github/workflows/cloud-smoke.yml`
- `.github/workflows/dependency-review.yml`
- `.github/workflows/devbox-runtime.yml`
- `.github/workflows/docs-quality.yml`
- `.github/workflows/docs.yml`
- `.github/workflows/hpc-registry-publish.yml`
- `.github/workflows/native-beta.yml`
- `.github/workflows/package-dry-run.yml`
- `.github/workflows/python-dependency-inventory.yml`
- `.github/workflows/registry-publish.yml`
- `.github/workflows/release-attestations.yml`
- `.github/workflows/release.yml`
- `.github/workflows/syft-linux-qualification.yml`
- `.github/workflows/toolchain-check.yml`
- `.github/workflows/validate-conductor.yml`

## 3. Repository Python Surface Classification (129 `.py` files)

- **Runtime Guards & Tooling (`scripts/`, `packaging/scripts/`, `tools/`, `website/scripts/`):**
  - Core guard: `scripts/python_runtime.py`
  - Packaging & release verification: `packaging/scripts/*.py` (15 scripts; `prepare_mainline_archive_release.py` retains explicit `3.14.8` Syft verifier container exception with rationale)
  - Supply-chain & NPM bootstrap helpers: `scripts/bootstrap-node-tools/*.py`, `scripts/supply_chain/*.py`, `scripts/maturity_tasks.py`, `scripts/agent_session.py`, `scripts/check_minimal_features.py`, `scripts/validate_dependency_identity.py`
- **Bindings & Gym (`bindings/python/`, `python/kairo_gym/`):**
  - `bindings/python/kairo_ecs/*.py` (8 modules) + `bindings/python/native_tests/test_native_engine.py` + `bindings/python/tests/*.py` (11 test modules)
  - `python/kairo_gym/src/kairo_gym/*.py` (3 modules) + `python/kairo_gym/tests/*.py` (2 test modules)
- **Benchmarks, Cloud, K8s & Notebooks (`benches/`, `cloud/`, `k8s/`, `notebooks/`):**
  - `benches/**/*.py` (8 files), `cloud/validate_cloud_hpc.py`, `k8s/operator/*.py` (2 files), `notebooks/*validate_notebooks.py` (2 files)
- **Unit/Policy Test Suites (`tests/`):**
  - 34 `.py` files covering runtime policy, packaging, archive supply chain, Syft, and maturity tasks.
- **Historical / Evidence Scripts (`conductor/`, `docs/distributed/`, `docker/`):**
  - 15 `.py` files retained as historical/evidence scripts (not active CI runners).

## 4. Safe Terminal Handover Criteria (Task 0.3)

1. `.venv/bin/python scripts/python_runtime.py` exits `0` with `version: "3.15.2"` and repo-local prefix.
2. `tests/test_local_python_runtime.py` and `tests/test_python_runtime_policy.py` pass without relaxing version or symlink guards.
3. No active workflow restores an unqualified older Python runner (`test_active_ci_does_not_restore_older_python_runners`).
4. Any alternate compatibility profile (e.g., `3.14.8` Syft verifier) is explicitly documented with scope and reason.
