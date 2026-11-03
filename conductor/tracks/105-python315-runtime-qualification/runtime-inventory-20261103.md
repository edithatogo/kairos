# Track 105 runtime reconciliation — 3 November 2026

This is a bounded inventory and evidence reconciliation at main
`0f5da40aadb76c06bc3b49d259e4713d2d1b56f2`, not a track closeout. The selected
profile is final CPython 3.15.2, with a repository-local `.venv`; the runtime
guard checks both the interpreter and its resolved prefix. Necessary alternate
profiles remain subject to the recorded consumer-specific qualification policy.

## Inventory and dispositions

- Development and automation: `.python-version`, `devbox.json`, `scripts/python_runtime.py`,
  bootstrap scripts, repository Python helpers, and the explicit subprocess
  interpreter contract remain in scope. The checkout inventory query found 119
  tracked candidate Python, notebook, lock, package and runtime files; this
  count is a search boundary, not proof every external consumer is qualified.
- Packaging and bindings: `bindings/python/pyproject.toml`, `python/kairo_gym/`,
  `templates/bindings/python/`, `packaging/scripts/` and the bootstrap, archive,
  package and Gym hash locks require the selected runtime. The Syft verifier's
  platform locks remain separate. Existing legacy 3.14.8 and 3.15.1 receipt
  aliases in archive/Syft verification validate historical provenance and are
  not current runtime selections.
- CI: 21 workflow files contain Python or local-environment references. Exact
  main binding/Gym/archive, inventory and Ubuntu/Windows Conductor jobs are
  linked below. Publication and security workflows retain their existing
  separate execution and authorization gates.
- Docker and Devbox: `docker/Dockerfile` and `devbox.json` select 3.15.2.
  Hosted native runs at `2530776` proved the container executable and prefix,
  plus absent/repeated Devbox startup. The Devbox configuration, Python profile,
  guard and workflow inputs were unchanged through the baseline main commit.
  Docker also builds the Rust source tree and `Cargo.lock`, which did change;
  the old container run is baseline evidence, not current-tree proof.
- Historical planning, prior release receipts, changelog history and synthetic
  wrong-version tests are not
  active supported-runtime declarations. They must not be rewritten as if their
  original runs used 3.15.2. The active Syft runbook instructions were corrected
  separately while retaining their dated historical receipts.
- Other checkouts, credentials, provider kernels and environments retain their
  native owners. An unrelated dirty toolchain checkout has overlapping files;
  its Syft runbook working bytes matched main before the independent runbook
  correction. No files or environments in that checkout were changed.
- The five executable root notebooks remain active documentation assets. The
  scheduler and benchmark tutorials use the selected Jupyter kernel without a
  verified local-prefix guard; three accelerator notebooks execute on a
  Colab-managed kernel and may install JAX. Their `3.12` metadata does not prove
  the actual kernel version or qualify an alternate profile. The docs-quality
  validator checks structure and AST, not cell execution. Runtime selection,
  dependencies and actual notebook execution remain open for these consumers.
- `cloud/gcp/submit-experiment.sh` previously rendered GCP array jobs with
  ambient `python`. Candidate `6bb072665c3fd5b43c8d28928734895b4163e30c`
  verifies the local interpreter before rendering; four POSIX tests use a fake
  `gcloud` to check success and fail-closed behavior. Hosted and real provider
  qualification remain separate, and no GCP job was submitted in this audit.
- Thirty-seven tracked helper shebangs name `python3`. Many rely on the caller's
  documented activation or an already-guarded workflow rather than enforcing
  a prefix themselves; each standalone consumer needs an explicit disposition
  before a complete active-use inventory can be accepted. The distributed
  qualification controllers check the version but rely on their documented
  local-venv invocation for the prefix; the remote helper independently checks
  both. Prior native qualification was not rerun by this documentation audit.
- `scripts/release/publish-registry.mjs` and `scripts/release/publish-hpc.mjs`
  spawn ambient `python` when called directly as documented. Hosted workflows
  select the local runtime before invoking them, but direct entry points do
  not prove that contract. The registry helper also requests unpinned upgrades
  of build tools. These release-owned paths require a scoped handoff and review;
  neither publishing path was executed.

## Verified evidence and remaining gates

- [Main bindings run 41618182701](https://github.com/edithatogo/kairos/actions/runs/41618182701): Python 3.15.2 binding, Gymnasium and archive jobs passed at `0f5da40`.
- [Main Conductor run 41618182892](https://github.com/edithatogo/kairos/actions/runs/41618182892): Ubuntu and Windows jobs passed at the same commit.
- [Main inventory run 41618183020](https://github.com/edithatogo/kairos/actions/runs/41618183020): four installed profiles submitted successfully and all 51 expected package identities were present in native readback. It does not prove dependency edges or extras.
- [Devbox run 41313320248](https://github.com/edithatogo/kairos/actions/runs/41313320248) and [Cloud/HPC run 41313320333](https://github.com/edithatogo/kairos/actions/runs/41313320333) passed on `2530776`, with the source-equivalence limits above.
- At source commit `3b0372ea8efcc25c4b4ae5867c5427f455696eaf`, cwd `/Users/doughnut/.codex/worktrees/kairos-track105-closeout/kairos`, `.venv/bin/python scripts/python_runtime.py` returned final 3.15.2 and its own local prefix, exit 0. `.venv/bin/python -m unittest tests.test_local_python_runtime tests.test_dependency_inventory_workflow tests.test_python_runtime_policy tests.test_verified_syft_installer tests.test_syft_installation_receipt` ran 68 tests, exit 0. Raw private runtime and test logs have SHA-256 `52f44daac45f92c61a23647b472ae492bd9e8de76c8508291b020045bf64d98e` and `696f6d399c4dba2a1edfc650178ba507c06ec37fa0c6ed84fcb512c9e8710cb6`, respectively, under `~/.codex/private-artifacts/kairos-track105-closeout-20261103/`.
- The 3 November Colab CPU capability probe returned `docker_path: null` and its isolated session terminated. The local Docker executable is a dangling OrbStack link and Colima is stopped. Neither observation is container execution evidence.

The current candidate intentionally triggers the existing hosted Cloud/HPC
container workflow via its evidence documentation and the native Devbox workflow
via its own trigger change. Actual final-head runs, normal protected merge,
exact-main hosted runtime readback, remaining external/owner inventory and safe
handover review must be recorded before closure. No environment or consumer pin
is promoted by this document.
