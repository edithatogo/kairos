# Combined C2.0 and accepted C4 qualification

Source `cebff2def1f64f9f8f119742cbe84a10ba2e9571` joins C2 preparation52a478b with accepted
C4 sourcefd21188. Every incoming C4 blob is identical to its accepted source;
C2 fixture/contracts are preserved. This avoids regressing the current parent
pin after parallel C4 publication. Earlier standalone C2 evidence remains
historical; combined-head qualification is required for parent reconciliation.

All six expected-red native runners return zero after declared Cargo101 missing
API failures. Raw native logs are retained here with verified hashes, along with
exact receipt/source/toolchain/fixture bindings. No behavioral test or C2 runtime
acceptance is implied. Original provider/hook/route/parity/transit/preemption
oracles and reviewed definitions remain unchanged.

Two existing accepted C4 evidence logs contain trailing blank lines; full staged
whitespace check returned2. Logs and their hashes were preserved unchanged.
The remainder of the staged diff passes. Exact details are recorded separately.

Combined-head hosted CI, native owners and parent reconciliation remain pending.
C2.0 checkbox and C2.2 dispatch wait for those gates. Preserve C4 accepted statuses,
Track49, API/MSRV/release/portable-checkpoint boundaries and ED MVP scope.
