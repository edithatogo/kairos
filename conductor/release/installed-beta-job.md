# Native beta installed-artifact qualification

Build both native targets from the exact tested commit. Produce the locked
Rust SDK source archive, the target-specific C ABI archive and the Python
wheel. Record their SHA-256 hashes, source commit, compiler, platform and
Python version in the qualification receipt.

In an independent directory, verify the archive and wheel hashes before
installation. Unpack the Rust SDK and C ABI, and start a new CPython 3.15.2
environment with no inherited packages. Install the wheel with no index or
dependencies. Run each check with the installed file paths.

- Build and execute the standalone Rust service-station model with the
  installed SDK. Record Cargo resolution and verify that the Kairos crates
  resolve beneath the installed SDK and the pinned dependencies match its lock.
- Compile a C program against the installed header and run it with the
  installed shared library. Check the dynamic library identity, C layout,
  dispatch ordering, error statuses, reset, free and buffer ownership.
- Run the existing Python native lifecycle suite against the installed
  distribution. Verify the adapter loads from the new environment and
  executes the installed C ABI library. Require typed rejection for missing
  configuration, wrong architecture, missing symbols and an incompatible ABI.
- Verify the Apache-2.0 declaration in the SDK and installed wheel and the
  full licence text in the Rust and C archives.
- Fetch the exact public CareOps commit named in the profile. Install a
  fresh `careops-core` source copy in a small consumer workspace and select
  the same installed Rust SDK. Execute the original
  `allocation_kairos_conformance` case against that SDK and retain its input
  and lock hashes. The test covers genuine claim/grant identities, contention
  and release through the native staged allocator.

The runner retains receipts and candidate artifacts through GitHub Actions
for review. Registry upload and release publication follow their own
authorised workflow. Upgrade and rollback across candidate versions are part
of the release-candidate phase.

## Reuse the tested bytes for provenance

Track 113 selects one successful `main` installation run and pins both
installation receipt hashes. [Candidate record](../tracks/113-beta-and-stable-release/metadata.json).

Release Attestations downloads those two bundles and checks their contents
against the pinned receipts. It scans the bundled SDK source and lock inventory,
C package contents, and Python package metadata with the verified Syft release.
It signs the SPDX document against the original package digests and retains the
bundle, subject checksums and installation receipts as `native-beta-provenance`.
[Workflow](../../.github/workflows/release-attestations.yml).

Run the workflow on `main` with `gh workflow run release-attestations.yml --ref main`.

Use `packaging/scripts/qualify_native_beta.py --output DIRECTORY` from a clean
checkout with the profiled Python and the hash-pinned packaging tools from
`scripts/package-python-tools.lock`. The output directory must be new. Its
`artifacts` directory holds the three distributions, checksums and the
installed receipt. The Rust SDK supplies the provider's locked dependency
versions when resolving both Cargo consumers.
