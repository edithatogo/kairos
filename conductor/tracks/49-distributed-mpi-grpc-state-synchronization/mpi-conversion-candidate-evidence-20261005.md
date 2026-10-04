# Track49 private MPI maintenance-mitigation candidate — 5 October 2026

Disposition: candidate, not adopted. The private experiment removes the maintenance dependency and passes its bounded checks. Production full freeze remains HOLD.

## Exact source and scope

Basee27425380f490d8013beefcdbbbdda2520fe5a9f. Graph-review input SHA-25607b062362ef85ac1fb1a95f3e0311ec7461c9511e4b288fddc30a59ce92fc2c5. Historical interface design stays unchanged at a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5.

Qualified distributed/security reviewer track48_external_interface_prepare cleared preparation, actual nine-file diff before execution, verification-target correction and actual results. Coordinator owns only the ignored private probe subtree, followed by this separately claimed evidence file. No original upstream source/cache, production manifests/locks, engine files, global status, parent pin or parallel queue/calibration paths changed. No third-party fork publication or upstream communication.

Local probe: /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004/artifacts/track49-mpi-conversion-probe-20261005. Its private source comes from verified mpi0.8.0 package checksum677762a4bde2c81158fc566a69b97d11b0c3358694e64f4f922ac5189be311cc. Full candidate.patch SHA-2566be05c795804347f5b194e63104cc974f91a8321b8f22797b34ab9fd465afdac and per-source-file hashes bind the experiment. Source VCS identity and actual compiler/OpenMPI versions are retained in environment.json.

Nine-file patch: remove conv from private normalised Cargo.toml; change six internal imports; add one crate-private module declaration and one sealed integer-only conversion helper using standard TryInto. All19 existing value_as call sites remain unchanged. No new public module/export/trait/signature appears. The helper accepts only sealed primitive integers, excludes floats/arbitrary conversion types, and preserves checked acceptance/rejection in the tested boundary cases.

Standard TryInto errors have different Debug formatting from conv errors. Preserving expect text does not preserve complete panic text. Exact panic-text compatibility remains unproven; no automatic unchanged-error claim. No inspected conversion error crosses a public return type, but complete API/trait behavior remains a separate review gate.

## Executed proof and retained setup failure

Root runner: /opt/homebrew/bin/python3 artifacts/track49-mpi-conversion-probe-20261005/run.py, cwd isolated worktree. Matching existing rustc/rustdoc selected per toolchain; own Cargo cache/target/temp, explicit SDK/CC/MPICC/libclang and existing read-only private SQLite inputs. Full absolute command/cwd/start/end/exit/log/patch/lock hashes retained in receipts.json. Bounded subprocess groups are reaped or killed on timeout.

First resolution succeeded, then cargo test -p mpi --lib exited101 before compilation: dependency package could not be tested outside workspace membership. attempt1 preserves original packet/runner/receipts/logs. Reviewed correction adds a test-only module in the private consumer that includes the exact helper source by path, without workspace/dependency change. These are helper-source tests, separate from actual MPI binary builds.

| Executed check | Actual result |
| --- | --- |
| Cargo1.99 offline candidate resolution | 0; 172 lock packages, 170 registry packages |
| Rust1.76 locked filtered helper-source tests | 0; five passed, zero failed |
| Rust1.76 actual candidate consumer binary build | 0 |
| Rust1.99 locked filtered helper-source tests | 0; five passed, zero failed |
| Rust1.99 actual candidate consumer binary build | 0 |
| OpenMPI5.0.11 real2-rank smoke, Rust1.76 binary | 0; both rank outputs verified |
| OpenMPI5.0.11 real4-rank smoke, Rust1.76 binary | 0; all four rank outputs verified |
| Strict cargo-audit0.22.2 | 0; zero reported vulnerabilities/warnings |

The five helper cases check negative Count-to-usize rejection, zero, maximum Count round trip, usize-to-Count overflow and small round trips without huge allocations. Actual binary calls patched MPI slice/array counts and library-version conversions; collective smoke asserts rank-sum all-reduce, exact all-to-all received rank sequence and fixed three-element broadcast on every rank. These are real MPI processes, not mocked rank values. Commands use existing /opt/homebrew/bin/mpirun --oversubscribe -n2/-n4; no cluster, scheduler, distributed-runtime driver, migration/cut/GVT or node-failure acceptance follows.

New lock removes conv/custom_derive and replaces registry MPI with the private path candidate; every other registry name/version/checksum equals the prior locked graph, asserted before tests. The candidate remains experimental; this lock is not the production lock or a downstream consumer-resolution proof. Runtime binary SHA-25680c2b1f8ce187224f7a531530f3c6ff00717b4d5ded9eccbb3e00a73a4bf8d2f.

Strict audit uses an owned copy of the official RustSec database at ef6173cbc5c50ec8166f9a5b28f07834144373ee with --no-fetch --deny warnings --json. The original registry-MPI graph's unmaintained finding is retained; this different private graph's clean result does not waive it. Candidate locked/offline metadata and cargo-deny0.20.2 under the unchanged repository policy also exit0 for licences/sources, with only the unused Unicode-DFS-2016 allowance warning.

## Independent acceptance and adoption boundary

Reviewer verified patch/input bindings, all eight execution log hashes, binary hash, unchanged other registry versions/checksums, every expected MPI rank and actual audit. CLEAR for this separate candidate evidence record. The viable candidate is confined to macOS aarch64/OpenMPI5.0.11 with these feature selections.

Remaining work before adoption: exact panic/error/public-trait compatibility decision with comparative tests, all conversion paths, minimum/current downstream consumer resolution, native source/provisioning/licence verification and supported-platform builds/runs. A reproducible source-delivery/maintenance plan must bind the eventual exact upstream revision or patch; no production path fork, package publication, MSRV increase or adoption is authorised by this record. Full storage/interface/module/golden and live distributed driver acceptance remain open. This proof supports a concrete reviewed mitigation choice, not automatic Track48/49 completion.

Local artifact inventory follows; raw outputs and candidate source are retained locally, not claimed as hosted artifacts.

| Local artifact | SHA-256 |
| --- | --- |
| packet.json | 1ddef1bcac02042ecc19b22aad095a6f31fd2764e33a833519a148b647de07ed |
| candidate.patch | 6be05c795804347f5b194e63104cc974f91a8321b8f22797b34ab9fd465afdac |
| run.py | 68c914a3b4623dbe099697a2ec6e66ccb496dd3ff95cd1167e21ffd8f36a9895 |
| environment.json | f23955c9785fddd8e04bb66d343c3ce97b0cd6f0e8ee52ce9adcac2cc295f0ec |
| receipts.json | a9edf49b8665e5edaaf9b2711cd2e2b108b655718b850fbf2832480ca3c82d56 |
| summary.json | 2f95590b6d6098a29d10497f05e98328b13c12c965e85ff2d2b13696623e0625 |
| scratch/Cargo.toml | b0ef12e3955cfb25cedfb21fa2f214da0be3c06b382d1d6203956f246b906257 |
| scratch/Cargo.lock | 4aa2337363a4f0a417d407fa61b7c9a432e6395f31815909648e72152d4ae73a |
| scratch/src/lib.rs | c30336309a6ed5e379cf337151999e88ccba1242f9aa1cca450e102396c18d24 |
| scratch/src/main.rs | 6124c4fa73a839d02c287cbd9758bd10cf18f9c972580404fa45308c4b137c45 |
| mpi-candidate/src/integer_conversion.rs | 381780c68baaf7a0d5361a1ad9bbce4f32762018d618d5d358027f7bc18e2de2 |
| tests-1.76.0.log | 83b5bde1fce76b8429fcfdbba95caeee1591b4e5c92ab7044360a31e669f6580 |
| tests-1.99.0.log | 3337d96df765c6df8f42ae59cacb95eb1e4b8aa9f0e074b19349fe7b1b24d365 |
| mpi-2.log | 036316cf5f72e54d37540b28ee717f1801587a9754114604596cd9989531988e |
| mpi-4.log | c1ddc316ed6f7bcd63d77cd6eaad1c94637bbac2ec48cd9603f273dafb2860cf |
| audit.log | cb6023b2ebd5a170a19ca265c5cec13d929e21711ee7f5f1d088bc6546a9ba41 |
| policy-receipt.json | 1bfe64aae50d021db6f89c7f602ac7e0033b29a1281702542935db592529a613 |
| policy.log | 1957f9df9caf070bab8c16706e6138c3b67c5b195180af674e0c4ab17e67cf58 |
| attempt1/receipts.json | 92aa9a76d3d1dde18ff67e6067605a34de3842ab657efa524fced95fa4291b37 |
| attempt1/tests-1.76.0.log | 69ddb9ec77fd4820745f2ed5a7b8b70fcc21fd62355deff9391323762b3b08eb |
