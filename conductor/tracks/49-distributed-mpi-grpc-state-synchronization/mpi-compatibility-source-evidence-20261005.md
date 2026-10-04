# Track49 MPI compatibility and source evidence — 5 October 2026

Disposition: reviewed private candidate evidence; production adoption and full contract freeze remain HOLD.

Base: 38ff05ef6f678c89ef07c0e2453113d3272734be. Previous candidate record SHA-256: 48363a16fb06667872b86c14c198c79b02584fea169573eb8dc15d17d8085dd8. This separately owned record preserves the original experiment and historical interface design. No production manifests, locks, parent pin, global status or parallel calibration paths changed.

## Candidate and compatibility scope

The revised nine-file private patch has SHA-256 ecc8b4f82282ac729c99b5c9818110cda29d81eac87f0499de5da9091ed0df54. Its upstream MPI 0.8.0 archive checksum is 677762a4bde2c81158fc566a69b97d11b0c3358694e64f4f922ac5189be311cc; source VCS revision b2ca71de414c68ad09eedbe7dd46059494c6b170.

Remove conv from the private normalized manifest, change six private imports, add a private module and checked helper. All 19 value_as sites remain unchanged. The sealed helper supports only i32-to-usize and usize-to-i32, using standard checked conversions. Private overflow Debug formats preserve the legacy NegOverflow(..)/PosOverflow(..) payloads for these tested pairs. No public export or signature changed in this diff; complete consumer/API compatibility is not certified.

The separate reference fixture intentionally retains conv 0.3.3 and custom_derive. Its legacy-comparison graph is not the clean mitigation graph. Candidate resolution contains 172 lock packages, including 170 registry packages, with conv/custom_derive absent and other registry versions/checksums unchanged from the prior candidate graph.

## Actual executed checks

Runner: /opt/homebrew/bin/python3 artifacts/track49-mpi-compatibility-20261005/run.py, working directory /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004. Exact argv, cwd, UTC timestamps, exit statuses and input/log hashes are retained in receipts.json. Private caches/targets, explicit matching rustc/rustdoc, native SDK/MPICC and read-only private SQLite inputs were used. All 12 commands exited 0; every receipt log hash was rechecked before this record.

| Check | Result |
| --- | --- |
| Offline candidate resolution | 0 |
| Rust 1.76.0 helper-source tests / comparison tests / actual binary build | 0 / 0 / 0; five helper and four comparison tests passed |
| Rust 1.99.0 helper-source tests / comparison tests / actual binary build | 0 / 0 / 0; five helper and four comparison tests passed |
| Separate reference resolution on each toolchain | 0 / 0 |
| Real Open MPI 5.0.11 two-rank / four-rank smoke | 0 / 0; every expected rank verified |
| Strict cargo-audit 0.22.2, deny warnings | 0; zero reported vulnerabilities and warnings |

Comparisons bind the tested macOS aarch64 ABI: Count/c_int are 32 bits and usize is 64 bits; actual RSMPI_MAX_PROCESSOR_NAME and RSMPI_MAX_LIBRARY_VERSION_STRING are c_int. They compare successful boundary values, negative and positive overflow Debug values, and selected expect/unwrap panic payloads against legacy conv. This establishes selected message and value parity on these toolchains and ABI, not every caller, optional feature or supported platform.

The Rust 1.76 binary performs rank-sum all-reduce, exact all-to-all rank receipt, three-element broadcast and public array count checks. Each rank asserts and prints the actual Open MPI v5.0.11 library identity. Binary SHA-256: 7bb594469b85e756a5fd5a703ca5fe90b6a3d486508e54e7d798619f36589bfb. This is native collective smoke, not distributed driver, cut, migration, recovery or failure acceptance.

Audit uses the owned official RustSec database copy at ef6173cbc5c50ec8166f9a5b28f07834144373ee with --no-fetch --deny warnings --json. The original upstream graph's maintenance finding remains recorded; no waiver follows from a different private graph's clean result.

## Reproducible source reconstruction

A fresh provider directory was constructed from the checksum-verified published archive. The constructor rejects absolute/traversal paths, wrong archive prefix, links and duplicate members; it copies regular members without extractall. After verifying the exact patch checksum, /usr/bin/patch --batch --forward -p1 -d <provider> -i <candidate.patch> exited 0. The actual absolute argv and timestamps are in provider-receipt.json. This constructor was executed inline; no saved builder script is claimed.

All 82 regular files exactly match the tested candidate, excluding only .cargo-ok; no symlinks, extra, missing or different files were accepted. Sorted compact JSON of relative file names and SHA-256 values hashes to d825dc616107164ca43d74914c7da859ce567a40818643c645e0c80b79d5c58b. LICENSE-APACHE hash c6596eb7be8581c18be736c846fb9173b69eccf6ef94c5135893ec56bd92ba08 and LICENSE-MIT hash c753626948da51cec25ba3fdfc311b182ce1861b66a1fcc52ed622307dc22020 are retained. This proves this source reconstruction, not production vendoring, publication, native provisioning or downstream propagation.

Qualified distributed/security reviewer track48_external_interface_prepare independently checked the execution receipts, exact patch, ABI assertions, all MPI ranks, audit, full provider map and license hashes, and cleared a separate evidence record. Final record review is recorded in delivery artifacts.

## Remaining gates

The wrapper still declares mpi-sys ^0.2.2 while these roots select exact 0.2.4. Minimum and current downstream consumer resolution therefore remain an explicit acceptance gap. A separately reviewed task must test the minimum or bind a justified private dependency minimum change and its consumers. No Rust floor increase or production manifest change is implied.

Native source/provisioning and supported-platform evidence, complete API/feature compatibility, storage commit-uncertainty/checkpoint acceptance, concrete module ownership and independent canonical golden fixtures remain open. Full freeze stays HOLD. Live gRPC/MPI distributed acceptance follows the accepted implementation contract. Calibration C4.2 writers retain their disjoint ownership.

Raw artifacts are local under artifacts/track49-mpi-compatibility-20261005, not hosted evidence. Their exact inventory follows.

| Artifact | SHA-256 |
| --- | --- |
| packet.json | e1d1b008d198f10405424da706aec5b55c7c0bdcbd260fba1e7b2109eba47305 |
| candidate.patch | ecc8b4f82282ac729c99b5c9818110cda29d81eac87f0499de5da9091ed0df54 |
| run.py | fbf26aade4624d6c1d25048460faee700a154ead611efafe2230555402f1d583 |
| receipts.json | aaa3b0a2104e68f94eeb2336b6ae8f31956fa720b14145bb42f8a9d4fcd2b46c |
| summary.json | b243aa817c1d4b7e4678c00f7de2fa4bfee34780123b1b66b9de322d8a6cccb3 |
| scratch/Cargo.toml | b0ef12e3955cfb25cedfb21fa2f214da0be3c06b382d1d6203956f246b906257 |
| scratch/Cargo.lock | 4aa2337363a4f0a417d407fa61b7c9a432e6395f31815909648e72152d4ae73a |
| scratch/src/lib.rs | c30336309a6ed5e379cf337151999e88ccba1242f9aa1cca450e102396c18d24 |
| scratch/src/main.rs | cb24e424b4bf1f65ae95d38a9c82b2e0de56a9c8eacf2028fa18d6824f6c9d8c |
| reference/Cargo.toml | 12a85513df515070c1cd4d0aa4253c46c0b894bca2547bcbfd8a944296fc8f21 |
| reference/Cargo.lock | e9297d875350a85c94d4be403e4df53df4dd5777d0fbe6260eabe70b594c96c0 |
| reference/src/lib.rs | 9a51dd0d294247c32c79e75046509eb807c1dcaa841834c07aa7e099dda120b6 |
| mpi-candidate/src/integer_conversion.rs | dd216824139bd5a333317d3a66fdb59bcfbbef0bd4b26376ee0ac55fe5b94af3 |
| tests-1.76.0.log | aab60ae555334ccf4df8e3fa0f41e891ac7fee88cccbe46716f6ca4110d918b7 |
| tests-1.99.0.log | f32c6fa274d2746602acbf469cf17f04e5fa6ab3d44e8ea7cd73634eb855d7aa |
| comparison-1.76.0.log | 926b6f8997d4b76ecb1f5467461abd71998cb350eaf1b15b728e63a25f390317 |
| comparison-1.99.0.log | f90d6832aab61e784121d42a46c20c03b09373d1d169696a560dc8aa48fd2ccb |
| mpi-2.log | 50724f133756fa11cf343bc06c045a5af983519e1f1d3fa7eb60ca5d63e19efc |
| mpi-4.log | a165494c609a66f54b49642557600d0ece3a5a8abb2b773bd9e8ad94b646cd3b |
| audit.log | cb6023b2ebd5a170a19ca265c5cec13d929e21711ee7f5f1d088bc6546a9ba41 |
| provider-receipt.json | 4668907c1dc42dd8e1b5a11a0e3bdefdd90b2691ab1be9559e018aed8aec37af |
| provider-patch.log | 959af1aede9d46a890b41d240ab96ec6692a5021a2a92693adca9232fce20a24 |
