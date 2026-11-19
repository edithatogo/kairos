# Grype dependency coverage

The Grype rollout adds a resolved source dependency inventory from the exact
`Cargo.lock` at the scanned Git commit. The shared actions in
[`repository-standards`](https://github.com/edithatogo/repository-standards) are
pinned to an immutable reviewed commit in each workflow.

## Policy and failure handling

Dependency PRs and weekly source checks are advisory for vulnerability findings.
Release or candidate preparation enforces the initial critical-severity threshold
before the existing publication or attestation steps. Missing or empty inventory,
scanner operational failure and invalid/stale vulnerability database fail in both
modes. The scanner retains its report, policy outcome, source/inventory identity,
tool version and database evidence even when the vulnerability gate fails.
Existing Rust audit and release qualification checks remain required.

## Inventory scope

Every resolved package from `Cargo.lock` is recorded, including development,
build, optional and target-specific dependencies. This intentionally broad source
inventory can therefore flag packages that are absent from a particular binary.
It does not establish binary composition or cover operating-system libraries,
independently resolved language bindings or downloads made by build scripts.
Git and path packages are retained with their source identity without claiming
a crates.io identity; vulnerability matching for these packages remains limited.

The source workflow records the checked-out commit rather than treating a tag or
branch name as immutable identity. Its optional manual `ref` permits a released
commit or tag to be rescanned with the current database. Weekly default-branch
checks cover current source resolution; they are not presented as rescans of
retained immutable released binaries.

## Qualification boundary

Local lock parsing and workflow validation are distinct from a successful hosted
Grype run. Required hosted CI and release qualification remain the acceptance
gates. No release was published, no dependency exception was introduced and no
existing vulnerability gate was disabled by this rollout.

References: [Cargo lockfile semantics](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html),
[Grype documentation](https://oss.anchore.com/docs/guides/vulnerability/getting-started/),
[CycloneDX JSON 1.6](https://cyclonedx.org/docs/1.6/json/).
