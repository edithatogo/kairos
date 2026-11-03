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
  validator checks structure and AST, not cell execution. Qualified runtime
  selection, dependencies and consumer handover remain open for these paths.
  A later isolated experiment executed the two local tutorials through actual
  Jupyter kernels on Python 3.15.2; details and limits are recorded below.
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

## Local Jupyter experiment — 3 November 2026

A disposable detached worktree at `b135ab85b54a2b753de6de85cf68932760ece2e6`
created a repo-local Python 3.15.2 environment and installed exploratory
`ipykernel==7.4.0` and `nbclient==0.11.0` with their resolved dependencies.
An environment-local kernelspec was registered under that worktree's `.venv`.
`NotebookClient` executed the four scheduler and three benchmark cells with
prepended kernel-side assertions of version and local prefix, and recorded the executable;
both notebooks passed. The environment is not a hash-locked or adopted project
dependency profile. The kernel warned about unencrypted TCP, and nbformat
warned that legacy cells lack IDs. The test does not qualify Colab accelerators,
the user's existing Jupyter environment, or notebook runtime handover.

The command used `.venv/bin/python -m ipykernel install --prefix "$PWD/.venv"
--name kairos105-qualification`, then invoked `NotebookClient` with
`JUPYTER_PATH="$PWD/.venv/share/jupyter"` and the worktree as notebook cwd.
The private execution log SHA-256 is
`b6ef42c423658d20d43c17f5cfd2f0327678cba2005119fcb5d4943dda872af5`, and the
resolved package list SHA-256 is
`27220a1a9cf2fabc9435d84eaf1a4e0b78d34529200d5c1facffb1c4398a0ab6` under
`~/.codex/private-artifacts/kairos-track105-closeout-20261103/`; both executed
notebooks are retained there. The disposable worktree was removed after its
kernel processes terminated.

An isolated Colab CPU probe reported provider kernel Python 3.13.18 at
`/usr/bin/python3`, prefix `/usr`, JAX 0.11.1 and one CPU device; its session
terminated. This is not a qualified 3.15.2 environment, a justified alternate
profile, or GPU/TPU notebook execution evidence. Colab runtime selection and
accelerator qualification therefore remain open.

## Colab runtime attempts — 3 November 2026

A T4 session at source `e33fc90d06318d089192eb15e8de3acc3552033a` could not
create the selected interpreter with the provider's `uv` 0.13.5. A separate
CPU diagnostic provisioned `uv` 0.13.7 in an isolated directory and created
Python 3.15.2. With that isolated bootstrap, a later T4 attempt created and
verified the local 3.15.2 environment and installed JAX CUDA tooling, but its
Jupyter kernel failed during startup because the provider's system IPython
configuration required the Colab kernel class. No GPU notebook cell completed.

A separate CPU experiment started a 3.15.2 Jupyter kernel by overriding the
provider-only class and extensions in its disposable kernelspec. Its TCP
transport warning remains unqualified. A later T4 attempt using that approach
lost its connection before producing a notebook or device result. The CLI
reported session termination and a subsequent session listing found none active;
there is no retained result proving the remote cell outcome. GPU and TPU
notebooks, device execution, package lock/security qualification and supported
provider-kernel handover remain open. No workaround was promoted into the project.

Private logs are retained under
`~/.codex/private-artifacts/kairos-track105-closeout-20261103/`. SHA-256 values:
initial GPU provisioning failure `18349a63946acb21e05b8c2740967819c5d2600086d68e4761b6780535337d67`;
GPU kernel-start failure `e8e743f3ea17402f0323dd14c60b5e44cc5f7d7dca4fe551c5cb337d099b1280`;
CPU kernel probe `7267857b8736759964f61f01349298c370c043045b8887223d0363ed3466dc5b`;
lost GPU connection `6de7701d4a7af9204c6a8eeb9eb950de5127dfb2cfcc3a820c9de7c1065c98f5`.
The environments were exploratory and not hash-locked project dependencies.
