# Agent Contract: cloud-agent

## Track

Track 39: Cloud / HPC Batch Runners

## Owned paths

- `docker/`
- `k8s/`
- `docs/cloud-hpc/`
- `.github/workflows/cloud-smoke.yml`
- `cloud/`
- `hpc/slurm/`
- `conductor/tracks/39-cloud-hpc-batch-runners/`

## Required handoff

- Summary of all container, operator, batch, Slurm, and telemetry artifacts produced.
- Docker image build instructions and multi-arch CI pipeline status.
- Kubernetes operator deployment steps and CRD lifecycle contract.
- Cloud batch provider job definitions and validation results.
- Slurm script usage examples with checkpoint/resume patterns.
- Spot resilience test results showing checkpoint integrity.
- Telemetry cloud output plugin configuration and checksum verification results.
- Follow-up items for experiment-agent (Track 22), packaging-agent (Track 15), and distributed-agent (Track 35).

## Handoff rules

### To Track 22 (experiment-agent)
- The Docker entrypoint wraps `kairo-ecs-cli` subcommands — must not alter argument parsing or exit code semantics.
- Checkpoint invocations use `kairo-ecs-cli checkpoint` and `kairo-ecs-cli resume` subcommands if available, or fall back to file-based state snapshot.
- Report any CLI gaps discovered during containerization.

### To Track 15 (packaging-agent)
- Docker image build consumes the release binary or builds from source depending on `--build-arg BUILD_MODE`.
- Multi-arch image tags follow `<version>-<arch>` convention. Coordinate tag strategy with release pipeline.
- Container image is a release-sidecar, not a replacement for library crates.

### To Track 04 (arrow-agent)
- This track reads Arrow telemetry output. It does not modify the schema.
- Cloud storage plugin writes Arrow files as opaque blobs with checksums. Internal schema compatibility is Arrow-agent's scope.

### To Track 35 (distributed-agent)
- The K8s operator and cloud batch definitions support single-node-per-job execution. Multi-node coordination (MPI/gRPC) is Track 35's scope.
- The `LogicalProcess` abstraction from Track 34 may be consumed by this track's operator for LP-to-pod mapping, but PDES-native orchestration is deferred to Track 35.

## Prohibited changes without ADR

- Adding, removing, or renaming `kairo-ecs-cli` subcommands or flags (owned by Track 22).
- Modifying the Arrow telemetry schema or serialization format (owned by Track 04).
- Altering the core ECS scheduler execution model (owned by Track 01).
- Changing the release packaging pipeline or crate publication policy (owned by Track 15).
- Modifying CI workflows other than `cloud-smoke.yml`.
- Committing cloud provider credentials or account-specific identifiers to the repository.
