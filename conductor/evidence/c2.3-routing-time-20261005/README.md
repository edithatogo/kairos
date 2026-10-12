# C2.3 routing and route-time — local leaf acceptance

Kairos source commit `d9607405b1c9a3998f7a9dbf4d623b0a07419787` contains the
accepted generic ABM spatial API and deterministic route-time implementation.
The coordinator accepted the `C2.3.routing` and `C2.3.time` leaves locally after
checking the exact source hashes, raw native receipts, immutable nine-case C20
runner, the source-bound route review, and the separate external C23 API smoke.
The implementation exports only `kairo_ecs_abm::spatial`; no dependency,
manifest, lockfile, or fixture changed.

The preserved raw receipts show: all nine C20 route cases passed at the final
source commit; default-feature ABM tests passed on Rust 1.76; strict all-target
Clippy passed on Rust 1.99; and all three external API/golden/mode-identity
smoke cases passed against an archive of that exact commit. The first committed
implementation used exponential simple-path enumeration; it was superseded by
`d960740`, which uses best-label Dijkstra. The first attempt logs remain
preserved under `worker/attempt1/`.

This is local leaf acceptance only. Full C2.3 remains open for actual transit
progress/carriers and dispatch integration. Hosted exact-head checks, parent
integration, stable API/release acceptance, full C2.0 runtime acceptance,
checkpoint/restore, and ED operational acceptance are not claimed here.
