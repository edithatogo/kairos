# Source candidate preparation — 2026-10-01

This is a local source rehearsal for the planned `0.4.0-alpha.1` product, not
an RC-ready registry set or published release. Source baseline:
`c369ed1ac5111290a1e18974e6d282329cf70959`. No version bumps, tags,
release publication, registry writes or production enablement are performed.
Machine-readable receipts: [candidate-evidence.json](candidate-evidence.json).

## Concrete artifacts

The preparation worktree produces ignored local artifacts:

- `dist/candidate/kairos-source-c369ed1.tar.gz`: exact committed public source,
  generated twice with identical bytes; SHA-256
  `0ba31755442d5fffc0f2ad4ca675c814e2bd98831bd270aef39398bce1d6d06e`.
- `dist/candidate/package-versions.json`: actual versions for all 26 Rust crates.
- `dist/candidate/inventory-release-artifact-manifest.json` and
  `inventory-SHA256SUMS`: hashes of 34 package manifests across seven ecosystems.
- `dist/candidate/SHA256SUMS` and `candidate-evidence.json`.
- `dist/kairos-source-rehearsal-c369ed1.zip`: reviewed local evidence bundle.

Manifest inventory checksums describe source metadata. They do not describe
compiled libraries, wheels, npm/NuGet archives or platform binaries. The source
rehearsal has no artifact-linked SBOM or signed provenance; those remain blockers.
The local bundle is outside Git and not an uploaded GitHub release artifact.

## Reproducibility and reused evidence

Generate the source archive without uncommitted/ignored files:

```sh
git archive --format=tar --prefix=kairos-source-c369ed1/ c369ed1ac5111290a1e18974e6d282329cf70959 | gzip -n > kairos-source-c369ed1.tar.gz
python3 packaging/scripts/build_release_manifest.py --check
python3 packaging/scripts/build_release_manifest.py --version 0.4.0-alpha.1-rehearsal
python3 packaging/scripts/build_release_manifest.py --verify-existing
```

The observed archive was compressed with Python's `gzip.compress(..., mtime=0)`;
reproduce its exact checksum using the recorded Python/toolchain implementation.
The shell command preserves the source and deterministic timestamps, but a
compression implementation/version can produce different compressed bytes.
Verify extracted source and Git identity as well as archive byte checksums.

[Package Dry Runs 36813270279](https://github.com/edithatogo/kairos/actions/runs/36813270279)
succeeded at source `a1f2bd5096403b10db0c689570977f3da6615f3d`.
Exact Git blob comparison proves unchanged crate/binding/packaging/bootstrap
inputs at this baseline. Reuse those five successful package jobs rather than
repeat unchanged builds. That run covers Rust types and selected bindings,
not the whole Rust publication train or every native platform; it uploaded no
artifacts. The receipt and source equivalence are recorded in the JSON.

## Fixed offline gate and validation boundary

Track15 previously rejected every publication-manifest filename, including the
later Track42 checked-in dry-run configuration. The preparation fix permits only
that exact known path with its schema/stage, Boolean disabled-publication default,
dry-run policy, health floor and protected environment. It executes no publication
commands; unexpected/generated manifests still fail.

Four isolated fixtures verify the accepted existing configuration, rejected
production enablement, rejected unexpected manifest, and rejected string health-score values. The Track15 delivery
validator passes structural checks but reports missing attestation; that is not
a release-readiness pass. Local policy checks run on the preparation worktree
with this fix, rather than on the archived baseline's old validator. The JSON
binds the tested validator hash. Track status and phase completion do not change.

## Remaining decisions and gates

| Gate | Current evidence | Owner/action |
|---|---|---|
| Product/package mapping | Planned product 0.4.0-alpha.1; Rust crates use 0.0.0/0.1.0; bindings retain preview versions | Release and package owners choose independent versions versus a coordinated train; do not silently rewrite every manifest |
| Rust registry packaging | 39 internal path declarations lack version constraints; bench is publish=false | Package owners add reviewed dependency versions and publish order for the selected release set |
| Package metadata/names | 20 Rust crates lack descriptions; C# bridge lacks repository metadata; registry name/owner checks unverified | Resolve selected-package metadata and native registry ownership before publication |
| Artifact preservation | Previous hosted package builds retained no archives | Build/retain the chosen package artifacts on supported targets with checksums |
| SBOM/provenance | No hosted release/SBOM run; current attestation workflows do not acquire ignored dist inputs | CI/release owners connect actual artifact generation/download to SBOM/attestation, then verify artifact identity |
| Release approval | Compatibility, health score >=9.5, security/red-team, environment/publisher and release-owner evidence remain required | Complete existing Track42/44 and release gates; no exception is inferred from a passing rehearsal |
| Citation/archive | Unreleased metadata; prior source archived in Software Heritage | After an authorized named release, bind actual tag/version/artifacts and archive receipts; keep #91/#92 open until then |

This preparation makes the next release review concrete. It does not authorize
publishing, claim independent adoption, or fill missing evidence with planned tests.
