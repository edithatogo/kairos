# S11 Native GPU model qualification

## Additive DES on Apple Metal

The hardware-only constructor keeps entity values on the selected native
device across additive DES batches. The 65-entity fixture verifies modular
signed boundaries, four batches of 131 events, whole-batch input validation,
snapshot isolation and empty replacement. The mixed particle fixture
interleaves native ABM and DES dispatch. Injecting an invalid storage usage
produced a native validation error and required a fresh upload.

The selected device was Apple M1 Max on macOS 27.2.1 (26B131), arm64, with
Rust 1.99.1 (58cf8f9c3) and wgpu 30.0.1. The baseline rejected the first DES
batch after a successful hardware particle step. The candidate passed 44
all-feature tests and 30 default-feature tests. The release timing fixture,
focused Clippy and rustdoc also exited zero.

The native buffer-counter probe first completed the device's initialization
copies. The original upload path retained 135 live Metal HAL buffers after
32 repeated uploads without dispatch or readback. After the fix, the probe
observed three initial buffers, five after the first upload and five after
the 32 replacements. Empty-state swaps were included. The previous path
failed the bound with exit 101. The qualified path passed with exit 0.

```sh
cargo nextest run --locked --offline -p kairo-ecs-gpu --all-features
cargo nextest run --locked --offline -p kairo-ecs-gpu
cargo clippy --locked --offline -p kairo-ecs-gpu --all-targets --all-features -- -D warnings
cargo test --locked --offline -p kairo-ecs-gpu --features wgpu-backend,wgpu/counters --lib repeated_uploads_retire_native_mappings -- --nocapture
cargo test --locked --offline -p kairo-ecs-gpu --features wgpu-backend --release --test metal_mixed_profile -- --ignored --nocapture
```

The matched timing runs include upload, every ABM and DES step and final
readback. Pipeline setup was 18.617625 ms and measured separately. Each
configuration warmed up both backends, alternated order for twelve pairs and
verified exact integer sums, same-device replay and particle coordinates
within 2e-5 of the CPU. Seed initialization began at 20261123.

| Entities and particles | Events per batch | Batches | Pairs | CPU median ms | Metal median ms |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 65 | 131 | 4 | 12 | 0.003042 | 3.203146 |
| 4096 | 16384 | 16 | 12 | 0.753312 | 9.472791 |
| 65536 | 65536 | 8 | 12 | 2.291021 | 7.366542 |

The CPU finished first in all 36 pairs.
[Device API and error contract](../../../docs/adr/0023-native-metal-additive-des.md).

<details>
<summary>Raw matched timing samples, in milliseconds</summary>

```text
entities	events_per_batch	batches	pair	first	cpu_ms	metal_ms
65	131	4	0	cpu	0.002875	3.306500
65	131	4	1	metal	0.004416	3.317416
65	131	4	2	cpu	0.003042	3.392916
65	131	4	3	metal	0.003584	3.318375
65	131	4	4	cpu	0.002875	3.342708
65	131	4	5	metal	0.003917	3.300750
65	131	4	6	cpu	0.005875	3.105542
65	131	4	7	metal	0.006334	2.968000
65	131	4	8	cpu	0.002667	2.931541
65	131	4	9	metal	0.003042	2.926417
65	131	4	10	cpu	0.002625	2.999917
65	131	4	11	metal	0.002916	2.870667
4096	16384	16	0	cpu	0.902125	9.953750
4096	16384	16	1	metal	0.832459	9.699291
4096	16384	16	2	cpu	0.836583	9.676041
4096	16384	16	3	metal	0.764958	9.210625
4096	16384	16	4	cpu	0.768000	9.233084
4096	16384	16	5	metal	0.751791	9.340792
4096	16384	16	6	cpu	0.747334	9.475833
4096	16384	16	7	metal	0.744542	9.469750
4096	16384	16	8	cpu	0.749125	12.853625
4096	16384	16	9	metal	0.747792	9.268959
4096	16384	16	10	cpu	0.754833	9.706208
4096	16384	16	11	metal	0.746125	9.437000
65536	65536	8	0	cpu	2.248167	6.854125
65536	65536	8	1	metal	2.326625	6.745875
65536	65536	8	2	cpu	2.151042	7.415500
65536	65536	8	3	metal	2.425625	7.753333
65536	65536	8	4	cpu	2.376458	7.623667
65536	65536	8	5	metal	2.425333	7.747083
65536	65536	8	6	cpu	2.243208	7.415917
65536	65536	8	7	metal	2.283500	6.650459
65536	65536	8	8	cpu	2.218667	7.472958
65536	65536	8	9	metal	2.333500	7.068333
65536	65536	8	10	cpu	2.298541	7.317584
65536	65536	8	11	metal	2.267667	7.261541
```

</details>

SHA-256 values for the selected inputs and output logs.

| Path | SHA-256 |
| --- | --- |
| `crates/kairo-ecs-gpu/src/metal.rs` | `30065649622ea67ecae7fe51d982c136315ef9d6a6bbf1a617c75a42ad9ef3c3` |
| `crates/kairo-ecs-gpu/src/compute.rs` | `7c65a4b637d45e477b617887fb22212ebefd7dbfecc83a6d0e82e18b403e6c3c` |
| `crates/kairo-ecs-gpu/tests/native_metal_des.rs` | `37f9f2ccf76b73ed6650d41b98b744401d1797f638387c36d699cf9a97dbeb81` |
| `crates/kairo-ecs-gpu/tests/metal_mixed_profile.rs` | `4aa33b2fa40ec04357ec5d798d0066c443a6a8e1688e4256f5bf120ad5223db6` |
| `Cargo.lock` | `38970a6fe699dafebb720188530d88c022b53fd563bbe0cc9ad5dc7aa4167e07` |
| `production-test.log` | `3176a01b1c2789cd7052db7318dc31254d8013c9db960112553b8b5e89fb2e88` |
| `production-default.log` | `ba8296f027b84befc9aae34e78dc9a66a56ab4cef84761faea58b18c26c1f6c1` |
| `production-clippy.log` | `c20fbf3b651eb5279f8b91977f40e7796e88bdd712be800ab9ddca0b697e7953` |
| `production-profile.log` | `cb745968bc1c7bd1088cc9e0494bc9e4123967abde240900d04e4fdd3c73d1bc` |
| `production-doc.log` | `d528e690d35b7aafe5f78584d2efb456215e3d93fe9d15334ab0b680683f10e8` |
| `production-counters.log` | `dac9e8d96e13b309f84cad8cb5f990d145c95a95f5afde0ac0a8d08ce3893b62` |
| `counters-before-qualified.log` | `ce959cd2646b29f6809374ab9d16f7f05f3c25e8eb24fb757f176c6a43939023` |

## Initial particle qualification

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
