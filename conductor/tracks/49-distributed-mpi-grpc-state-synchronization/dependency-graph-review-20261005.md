# Track49 dependency graph review — 5 October 2026

Disposition: hold_for_evidence. Source/archive-member verification and candidate licence/source-policy checks pass; maintenance, platform/compiler, native provisioning and full interface/storage acceptance remain open. No dependency adoption, waiver or track completion.

## Scope and baseline

Base00b544b5024b24a9177c34ea46b66f2377de32f1; historical design baseline unchanged SHA-256a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5. Parent observed ba2f538597b88776ab1ca41ccc96a05f3ae1ad64 with Kairos pin8cd03c8f791ae58b33e5cc61b244071937a839ac; those are parallel development context, not this probe's source. No parent mutation or pin update.

User authorised continued Track48/49 readiness work. Qualified reviewer track48_external_interface_prepare reviewed this separate bounded packet and its private-cache/network fallback before execution. Root owned ignored artifacts/track49-graph-review-20261005 in isolated /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004, then acquired a fresh exclusive claim for this one evidence record. Production source, manifests/locks, global status and parallel Q/calibration files unchanged. Previous scratch source/cache/manifest/lock remain unchanged; fresh copied Cargo home contains new network inputs. Recipes and advisory inputs grant no adoption authority.

## Executed inventory and preserved failures

Existing Cargo1.99.0 metadata --format-version1 --locked --manifest-path <absolute prior scratch-alt3/Cargo.toml>:

| Command/result | Actual exit |
| --- | --- |
| --offline all-target metadata; missing target-specific crate bytes | 101 |
| Locked network fallback into owned copied cache | 0 |
| --offline --filter-platform aarch64-apple-darwin | 0 |

Command receipts preserve exact absolute argv, cwd, timestamps, exits and output hashes. Lock SHA-256 remains7f501e2cd0f485e7700df9be90a7bcea8f53dbd52bfd503741058d57a7e38cc8 throughout. No re-resolution or version change. Existing matching canonical Cargo is an inventory tool; it does not prove compiler compatibility for all targets.

First source-inventory script exited1 because it assumed Cargo1.99 extraction supplies .cargo-checksum.json. Failure retained in initial-source-inventory-failure.json. Corrected inventory-only retry reads preserved metadata and compares each published archive SHA-256 with the lock, then compares every regular archive member against the extracted source bytes. No archive extraction by this checker; traversal components reject.

- 173 registry packages; all archives available; zero checksum/member-byte errors across9,563 files.
- 152 host-reachable packages in filtered metadata, not an assertion each is compiled or used at runtime.
- 41 packages declare no rust-version. Prior actual Rust1.76 macOS build remains bounded proof, not inferred support for all undeclared packages/targets/features.
- Higher declared floors: target-only wasip2 1.0.4+wasi-0.2.12 requires1.87; wit-bindgen0.57.1 requires1.85. Both absent from this host graph. No claim of Rust1.76 for those targets.
- Seven duplicate package names: getrandom, hashbrown, itertools, shlex, socket2, syn, windows-sys. Duplicate versions retained, not forced into incompatible majors.

Coverage is archive-member byte verification, not complete extracted-directory integrity: extra files outside archive membership are not detected. inventory.json separates all-graph and host resolved features from published dependency declarations (target/build/dev/optional/default-feature edges). Declared optional/dev edges are not automatically resolved or compiled. The metadata graph is this 21-direct-pin scratch candidate, not a production package graph or minimum-version downstream consumer test.

## Actual licence/source-policy results

Existing cargo-deny0.20.2, exact repository deny.toml SHA-25617a514bd7c1b724632c841ea1a4ecd0113eb24d638588f92810db5207d0f76a7, --metadata-path <all metadata> --manifest-path <scratch manifest> --offline --locked check licenses sources: exit4. Sources passed; only licence error was the authored synthetic root lacking a declaration. No overall pass inferred; original log retained.

Separately reviewed follow-up copied unchanged lock/dependencies and authored source into licensed-scratch, adding only MIT OR Apache-2.0 to that synthetic package. No original manifest or metadata edited. Locked offline metadata exit0, identical dependency IDs/versions/features/edges and source declarations after excluding only the synthetic root's changed path ID; lock unchanged. Same cargo-deny policy check then exit0: licences and sources passed. Warning: unused Unicode-DFS-2016 allowance, not a licence failure. This checks SPDX/licence discovery and configured source policy for the candidate graph, not native-library licensing/provisioning or legal/production adoption.

## Fresh advisory finding and concrete mitigation preparation

Fresh depth1 clone of official https://github.com/RustSec/advisory-db.git into the owned subtree: exit0, revisionef6173cbc5c50ec8166f9a5b28f07834144373ee (unchanged from earlier observation). Existing cargo-audit0.22.2 through Cargo1.99 audit --file <unchanged licensed-scratch/Cargo.lock> --db <new advisory-db> --no-fetch --deny warnings --json: actual exit1, zero reported vulnerabilities, one unmaintained [RUSTSEC-2025-0058](https://rustsec.org/advisories/RUSTSEC-2025-0058.html). Full commands/revision/report hash retained. No waiver; strict scratch policy remains distinct from repository advisory policy.

Source trace: mpi0.8.0 uses conv::ConvUtil::value_as for slice lengths, MPI counts, dimensions and other numeric conversions. conv0.3.3 unconditionally imports custom_derive0.1.7 and generates conversion error types. Turning off MPI derive/user-operations cannot remove this chain; newer MPI alone retains it. An informational maintenance finding is not a demonstrated exploitable vulnerability, but it prevents declaring the strict candidate audit clean.

Recommended next mitigation packet: evaluate an exact, separately reviewed MPI source revision/patch replacing integer conversions with standard checked TryFrom and removing conv only after all its uses are accounted for. Preserve rejection of negative/out-of-range inputs, panics/error types/public API and trait behavior. No unchecked as casts or substitution of floating-point conversions. Bind upstream source/archive, patch hash, licence and declared floor; run conversion boundaries and public/API regression, minimum/current downstream consumer resolution, Rust1.76/current compile and real2-/4-rank tests. The patch/fork is not implemented or approved for adoption by this record; no third-party publication or upstream message authorised.

## Independent acceptance and remaining holds

Qualified reviewer independently checked metadata parity, receipt/output hashes, aggregate inventory, licensed policy and fresh audit. CLEAR for this bounded evidence record, with explicit archive-member and declared-versus-resolved limits. Full freeze remains HOLD: real native provisioning/provenance across supported hosts, target/feature MSRV and minimum/current consumer graphs, maintenance mitigation, production storage/recovery and accepted module/golden bindings remain open. Historical macOS transaction/process-kill results do not prove power loss, driver capability/replay or distributed recovery. No production implementation dispatch authorised.

Local retained artifact inventory follows; these files are not hosted CI artifacts.

| Local artifact | SHA-256 |
| --- | --- |
| packet.json | da61901605e2fdfa0481ff9fdeaa848fab8a88602b91efd01ffb99e9b98e4b11 |
| review.py | 2f6ea4c3a6e6bfb1fc58156c6b7fc95bac84cab167a7d5a53ec4e72f1ddc2371 |
| command-receipts.json | c7224230fa86222eb32e013da11090c494ec6356f1b150082e14468b604bd4d7 |
| metadata-all.json | b012fffaec8ee87386ddae674f6455e93f85fb1add3c5c16483158e084e4aac8 |
| metadata-host.json | e37e3ea92ad961705e716f46cbbfd3e80d706f3d1319e3efdd3043023657d3da |
| inventory.json | 2a4ab99317545822c3bb5983ae1ca9e82e55c6e26f0c220e6103430d89394d35 |
| summary.json | fd05870e36a71a4e2825a9bde83c46ab8d436de3b114fda0afb85336416a7f72 |
| initial-source-inventory-failure.json | a6b7a4360263c226e15d8b80977166ffebe2bcb1655b8f2db46a00ab56fce586 |
| policy-receipt.json | 907c1f44c3bf741ed1d44feb86303ba57733abd838222bebc71db830f6188f9b |
| policy.log | 7728c32f8c9e44fb4d3de6d73282f7374fb23fe14ca7e37a7c34585bafa079fe |
| licensed-scratch/Cargo.toml | 1be16e5809e1f51ed06d3e5b50c174ce64ae33e421c4b5e247f9add8e435bfd1 |
| licensed-parity.json | b4517a963a5d50cc3b7357ee3f341b4d98cc84e713e8897de41c2394a7ea26fe |
| licensed-policy-receipt.json | 0fdb68cca0247d69d9f8bcf7f6e21a87fbea8f001b1be6531bcd07bf83c30e92 |
| licensed-policy.log | 1957f9df9caf070bab8c16706e6138c3b67c5b195180af674e0c4ab17e67cf58 |
| fresh-audit-receipt.json | d25f64785b8847f6b01e7ce4b9716d1951c4e9c728df200b84bff59196de2d38 |
| audit.json | 7c7bc7a7cb9659539c8603f55d4923e147b0997ca89aa44de9130305d5bae735 |
