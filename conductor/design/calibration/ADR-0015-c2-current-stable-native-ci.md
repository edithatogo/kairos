# ADR-0015: Rust 1.99.0 current-development qualification profile

**Status:** accepted for the user-authorized internal C2 development
qualification profile only. This is not a language support-floor, release, or
public runtime-contract decision.

## Context

The current C2 implementation qualification is authorized to use Rust `1.99.0`
only. Existing matrix and CI expectations name Rust `1.98` and a beta lane. The
existing native-owner workflow tests default calibration features, which omit C2
flow execution; local C2 receipts already exercise the flow profile. Track 13
adds explicit hosted flow/demo coverage. Track 30 owns its matrix and validator,
while the `.github/workflows/toolchain-check.yml` implementation is a separate
Track 13 dependency.

The Cargo workspace and crate manifests contain existing `rust-version`
declarations. Those declarations are historical compatibility metadata here;
the current-development profile does not verify them.

## Decision

1. For the current C2 development qualification profile, use the pinned current
   stable Rust toolchain `1.99.0` only, with matching `rustc`, `cargo`, `rustfmt`,
   and Clippy from that toolchain. The Track 30 installed-version check uses the
   `1.99` version prefix.
2. Do not execute older or beta Rust runtime lanes for this profile. Do not change
   the existing Cargo `rust-version` declarations or claim older versions are
   discontinued, unsupported upstream, or newly unqualified by separate evidence.
   This profile makes no minimum-version change and starts no version-drop notice.
3. Keep prior receipts and their exact historical toolchains intact. Do not
   relabel their toolchain results as Rust `1.99.0` results.
4. Preserve all non-Rust Track 30 matrix rows, workflow assertions, and support
   rules. The matrix's Rust row describes this narrow execution profile and keeps
   the declared Cargo minimum explicitly marked as historical and unverified by
   this profile.
5. Integrate the Rust channel and expected-prefix change in the Track 13-owned
   workflow separately. Track 30's static validator will require channel
   `1.99.0` and expected prefix `1.99`; until that workflow change is integrated,
   the static matrix validator is expected to fail at the Rust lane check.
6. No public API, dependency, manifest, lockfile, engine behavior, or stable
   runtime contract changes follow from this decision. No platform support claim
   follows beyond the exact runner/toolchain evidence recorded for the profile.

## Verification contract

The bounded Track 30 packet updates the matrix, static validator, and test matrix.
The available local version check, with the pinned Rustup toolchain first in
`PATH`, is:

```sh
toolchain_bin='/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin'
PATH="$toolchain_bin:$PATH"
for tool in rustc cargo rustfmt; do
  tool_path=$(command -v "$tool") || exit 1
  case "$tool_path" in "$toolchain_bin"/*) ;; *) exit 2 ;; esac
  version=$("$tool" --version) || exit 1
  printf '%s: %s\n' "$tool_path" "$version"
done
case "$(rustc --version)" in *1.99.0*) ;; *) exit 3 ;; esac
case "$(cargo --version)" in *1.99.0*) ;; *) exit 4 ;; esac
```

The Track 30 static validator is run once before Track 13 workflow integration;
its expected Rust-lane failure is retained. Its `-CheckInstalled` invocation also
runs static checks first, so it cannot reach the installed-version check until the
workflow is integrated. The root coordinator runs the static validator again
after integrating the pinned `1.99.0` workflow lane, in addition to checking the
Track 13 worker's separate native CI changes. A passing local version check only
identifies the local compiler tools; it does not establish hosted CI, older MSRV
compatibility, cross-platform support, or C2 acceptance.
