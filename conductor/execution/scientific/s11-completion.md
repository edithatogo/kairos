# S11 Native GPU model qualification

The first implementation uses the existing Metal ABM kernel through wgpu 30.0.1.
It qualifies particle motion and buffer replacement on the selected Apple
Silicon device. The initial state and ABM steps use the public
`kairo-ecs-gpu` API. Run the locked `maturity_device_model` target with
`wgpu-backend` and Rust 1.99.1 on Apple Silicon.

## Device state and admission

Require a native Apple Metal adapter at the hardware-only entry point. Preserve
the resident particle buffer between submissions and copy the result on explicit
download. A successful upload replaces the previous particle state, including
when the new state has zero particles. Backend limits must reject an oversized
request before replacing the resident state. A native readback error must
surface to the caller.

The linked CPU implementation provides compatibility for the general
constructor. The hardware-only constructor rejects DES until there is a native
DES implementation. GPU transfer counters measure particle bytes and submitted
step parameters. Device buffers are owned by the runtime.

## Oracles and remaining cases

Use independent scalar position updates on 65 particles with unequal velocity,
fixed low-32-bit seeds and exact `vx` and `vy` checks. Compare positions within
`2e-6` over four steps and require exact same-device replay. Test an empty
replacement, final readback, and independent clone state. Compare end-to-end
upload, stepping and one readback on CPU and this device using paired timings.

Larger model semantics, native DES, other native backends and WebGPU in a real
browser require separate executable oracles. Performance claims are limited to
the measured model, device and timing boundary.

## Native readback

The selected wgpu adapter reported Apple M1 Max and a compute workgroup
limit of 256 threads. The macOS host ran 27.2.1 (26B131) on arm64 with
Rust 1.99.1 (58cf8f9c3). The baseline source was
`3befc81a3700db085a7973c7224b9bdce7077dc1`. It dispatched one workgroup
after an empty replacement and the new test exited 101.

The candidate passed 40 tests with the locked GPU crate's full feature set.
Two concurrent downloads returned the same state. Native wgpu rejected a
deliberately invalid binding, and a fresh upload restored the session.
The test observed native validation and recovery. Physical device loss
remains unqualified. The locked default build passed 29 tests. Clippy
with warnings denied and docs with rustdoc warnings denied both exited zero.

Independent review checked adapter selection, error scope ownership,
concurrent callbacks and clone behaviour. Review led to bounded callback
waiting, typed validation and allocation paths, and the requirement to
upload fresh state after a failed step. A clone captures an independent
CPU snapshot and can panic if its native readback fails. Use the fallible
state download when readback recovery is needed.

Source SHA-256 values for the Rust module, independent model test, existing
hardware test and workspace lock are recorded in order below.

```text
3d4de03e58c5297082bbaea2c49d36a697204ad79c9b0211c2fdb263d266f13f
fa75a4ec1b57e10102ad2243258acaddf19d8af8b2dc1780a26ea8c8d868ce64
67b22247d5bf5800f6e171f348708f57e0d050ac91eeaa623ba8054d62389311
69e513fd02e5c2e3f4ee2b57c9b17ab4c4bb4bf1b78dabdaeea444bce32c8ecb
```

The baseline and final test-log SHA-256 values are
`5f05292b490d878610286b5c11654a528810dfecf635d8bea800766fa42ac9a3`
and `a55404cd2b3276545f48fcf7d96ace1ea3202dd132640262f8571e989f2f4f21`.
Commands ran from the isolated Kairos root.

## Paired particle timings

```sh
cargo test --locked -p kairo-ecs-gpu --release --features wgpu-backend \
  --test maturity_device_model measure_native_abm_end_to_end -- --ignored --nocapture
```

Measurements include initial-state upload, all steps and final readback.
Each group used one warm-up per backend and alternating execution order.
The benchmark used deterministic initial positions and step seeds beginning
at 20261123. It compared the final positions after every pair. Pipeline
setup was 19.618 ms and sits outside the paired timings.

| Particles | Steps | Pairs | CPU median ms | Metal median ms | Metal wins |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,096 | 1 | 12 | 0.008313 | 0.662270 | 0 |
| 4,096 | 16 | 12 | 0.069084 | 5.192813 | 0 |
| 65,536 | 1 | 12 | 0.101583 | 0.964584 | 0 |
| 65,536 | 16 | 12 | 0.901021 | 4.795959 | 0 |
| 262,144 | 16 | 12 | 3.456854 | 6.190542 | 0 |

The benchmark exited zero. Its complete log SHA-256 is
`2c1e818e01abb86da7d8fdb10aad4a79be417a7e65e6bcc1aa6ed1a662e885b7`.
