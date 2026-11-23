# Native CUDA qualification on the Windows RTX 4060

## Runtime and recovery

The Windows x86-64 host used OS build 26300, an Intel Core i9-14900K, NVIDIA
driver 617.14 and CUDA toolkit 12.6.77. Rust 1.99.1 (58cf8f9c3) and the
locked cudarc 0.19.10 built the exact source in an isolated directory.
`nvidia-smi` reported an RTX 4060 with 8,188 MiB and compute capability 8.9.
The driver queries reported 8,585,216,000 bytes and capability 8.9.

The original bridge reported 16 GiB and capability 7.5 on the same device.
Its two native particle steps gave a maximum position error of
0.00018107891 against the seeded scalar reference. The corrected
257-particle run matched the reference and reported the actual driver
properties. The public hardware-only model passed both signed boundaries,
four batches of 513 DES events over 65 entities, invalid-batch rejection and
empty replacement. The recovery fixture marked particle state indeterminate,
verified the dispatch and readback fence, and restored execution with a new
upload. Each native fixture exited zero.

The Windows ordinary package suite passed 35 tests. The ordinary Apple
Silicon all-feature and default suites passed 44 and 30 tests respectively.
The native tests require the named hardware runs. Clippy and rustdoc exited
zero locally. [CUDA API and failure contract](../../../docs/adr/0024-native-cuda-mixed-model.md).

## Device kernel and copy trace

Nsight Systems 2024.5.1 captured the required-device model in a separate
CUDA-only trace. The native fixture passed while tracing. Its SQLite export
recorded nine kernels and the two final readback copies. The first device
readback occurred after the last kernel. The seven host-to-device copies
account for initial state and the five DES payloads.

| Device activity | Calls | Bytes | Total device ns |
| --- | ---: | ---: | ---: |
| Particle kernel | 4 | | 4352 |
| Additive DES kernel | 5 | | 5504 |
| Host to device | 7 | 20804 | 5152 |
| Device to host | 2 | 4372 | 2143 |

The instrumented process used the Windows hardware fixture and SHA-256
`29226012ca98c936c7e9675fd3c687b41b24131641872e23c8077e1bc5d017bc`. The profiler used
`nsys profile --trace=cuda --sample=none --cpuctxsw=none --stats=false`
and the `cuda_gpu_kern_sum`, `cuda_gpu_mem_time_sum` and
`cuda_gpu_mem_size_sum` reports. The report SHA-256 is
`89ecfecc99210ab9662355f380312f99bfbb6f56fc74b8d0241008239b773e29`. The exported SQLite SHA-256 is
`bed85a0ed761cbf5a52734861eace9184fecdcc657a5dc4ebcc397437ec647a3`.

## Matched local measurements

CUDA context and kernel loading took 66.666300 ms. Both backends warmed up
once per configuration. Twelve pairs alternated execution order with fixed
inputs and seeds starting at 20261123. Each pair verified identical integer
outputs and particle counts, coordinates within 2e-5, and exact replay on
the same device. Total latency includes upload, every prescribed step, and
readback. The speed ratio is the median paired CPU/CUDA latency ratio.
Parentheses show the 95% percentile interval from 10,000 paired bootstrap
resamples with seed 20261123.

| Particles and entities | Events per batch | Batches | CPU median ms | CUDA median ms | Paired ratio (95% interval) | CUDA first |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 65 | 131 | 4 | 0.002050 | 0.118650 | 0.018 (0.015–0.020) | 0/12 |
| 4096 | 16384 | 16 | 0.626950 | 1.113100 | 0.560 (0.554–0.569) | 0/12 |
| 65536 | 65536 | 8 | 4.900400 | 2.260500 | 2.261 (2.129–2.492) | 12/12 |

| Particles and entities | CUDA state upload ms | CUDA mixed steps ms | CUDA final readback ms |
| ---: | ---: | ---: | ---: |
| 65 | 0.014400 | 0.083500 | 0.012550 |
| 4096 | 0.024650 | 1.062550 | 0.024600 |
| 65536 | 0.380600 | 1.600900 | 0.290000 |

The step phase includes event uploads and kernel dispatch. Each result covers
the identified hardware, driver, toolchain and workload. The implementation
keeps device selection explicit.

<details>
<summary>Raw matched timing samples, in milliseconds</summary>

```text
entities	events_per_batch	batches	pair	first	cpu_ms	cpu_upload_ms	cpu_steps_ms	cpu_readback_ms	cuda_ms	cuda_upload_ms	cuda_steps_ms	cuda_readback_ms
65	131	4	0	cpu	0.001800	0.000100	0.001600	0.000100	0.126700	0.014700	0.090500	0.021500
65	131	4	1	cuda	0.002500	0.000200	0.002200	0.000100	0.144100	0.014100	0.108200	0.021800
65	131	4	2	cpu	0.002100	0.000200	0.001800	0.000100	0.103700	0.027200	0.064600	0.011900
65	131	4	3	cuda	0.002200	0.000100	0.002000	0.000100	0.144000	0.050800	0.080200	0.013000
65	131	4	4	cpu	0.001800	0.000100	0.001600	0.000100	0.087900	0.013500	0.062400	0.012000
65	131	4	5	cuda	0.002100	0.000100	0.001900	0.000100	0.116700	0.028500	0.076300	0.011900
65	131	4	6	cpu	0.001800	0.000100	0.001700	0.000000	0.090000	0.010000	0.068700	0.011300
65	131	4	7	cuda	0.002000	0.000100	0.001800	0.000100	0.108000	0.018300	0.077600	0.012100
65	131	4	8	cpu	0.001800	0.000100	0.001600	0.000100	0.115600	0.009800	0.086800	0.019000
65	131	4	9	cuda	0.002100	0.000200	0.001900	0.000000	0.120600	0.009900	0.089100	0.021600
65	131	4	10	cpu	0.001800	0.000100	0.001600	0.000100	0.127800	0.010600	0.105500	0.011700
65	131	4	11	cuda	0.003400	0.000300	0.002900	0.000200	0.136300	0.031600	0.088500	0.016200
4096	16384	16	0	cpu	0.644700	0.002500	0.640200	0.002000	1.148800	0.036100	1.087100	0.025600
4096	16384	16	1	cuda	0.657600	0.002400	0.644300	0.010900	1.141900	0.022100	1.094400	0.025400
4096	16384	16	2	cpu	0.643400	0.002900	0.614500	0.026000	1.159300	0.051500	1.082200	0.025600
4096	16384	16	3	cuda	0.635100	0.004600	0.626600	0.003900	1.113300	0.020400	1.067600	0.025300
4096	16384	16	4	cpu	0.611900	0.002000	0.606300	0.003600	1.136300	0.030500	1.080700	0.025100
4096	16384	16	5	cuda	0.618800	0.002500	0.612600	0.003700	1.090700	0.020800	1.045600	0.024300
4096	16384	16	6	cpu	0.639300	0.002000	0.634000	0.003300	1.110600	0.028300	1.057500	0.024800
4096	16384	16	7	cuda	0.636100	0.003300	0.629600	0.003200	1.139100	0.020300	1.094900	0.023900
4096	16384	16	8	cpu	0.609800	0.002200	0.604100	0.003500	1.112900	0.031500	1.057000	0.024400
4096	16384	16	9	cuda	0.605500	0.002400	0.600100	0.003000	1.092800	0.020300	1.048800	0.023700
4096	16384	16	10	cpu	0.614000	0.001900	0.609500	0.002600	1.107000	0.027200	1.055400	0.024400
4096	16384	16	11	cuda	0.617400	0.002200	0.612400	0.002800	1.086200	0.020200	1.042400	0.023600
65536	65536	8	0	cpu	4.637300	0.235900	4.222300	0.179100	2.310300	0.382000	1.627000	0.301300
65536	65536	8	1	cuda	4.810100	0.224500	4.393800	0.191800	2.136800	0.352500	1.494800	0.289500
65536	65536	8	2	cpu	4.739400	0.220800	4.334600	0.184000	2.252500	0.379200	1.600700	0.272600
65536	65536	8	3	cuda	4.916100	0.246200	4.468300	0.201600	2.165200	0.351000	1.536400	0.277800
65536	65536	8	4	cpu	4.874100	0.238900	4.448700	0.186500	2.251500	0.376200	1.601100	0.274200
65536	65536	8	5	cuda	4.795800	0.215300	4.388100	0.192400	2.079700	0.345400	1.459900	0.274400
65536	65536	8	6	cpu	4.884700	0.237000	4.459700	0.188000	2.268500	0.406900	1.571100	0.290500
65536	65536	8	7	cuda	5.528000	0.219600	5.056900	0.251500	2.213300	0.360900	1.570500	0.281900
65536	65536	8	8	cpu	8.794600	0.359900	8.174300	0.260400	3.355700	0.521200	2.441300	0.393200
65536	65536	8	9	cuda	6.842600	0.338500	6.232300	0.271800	3.562600	0.498600	2.642500	0.421500
65536	65536	8	10	cpu	7.779500	0.803400	6.771700	0.204400	3.113800	0.491100	2.275100	0.347600
65536	65536	8	11	cuda	7.715100	0.301500	7.159300	0.254300	3.102800	0.504700	2.172500	0.425600

```

</details>

## Reproduce

The selected source base was `d588ecd03c2210109bf13f4885d1125d5a66b9ad` with the CUDA source overlay.
The locked Windows runs used an x86-64 MSVC Rust 1.99.1 process. Run on a
host with an NVIDIA CUDA driver and the corresponding build toolchain.

```sh
cargo test --locked --offline -p kairo-ecs-cuda --features cuda-12000 --lib -- --ignored --nocapture
cargo test --locked --offline -p kairo-ecs-gpu --features cuda-backend --test cuda_native_model -- --ignored --nocapture
cargo test --locked --offline -p kairo-ecs-gpu --features cuda-backend --release --test cuda_mixed_profile -- --ignored --nocapture
cargo test --locked -p kairo-ecs-gpu -p kairo-ecs-cuda --all-features --lib --tests
```

| Input or run log | SHA-256 |
| --- | --- |
| `Cargo.lock` | `38970a6fe699dafebb720188530d88c022b53fd563bbe0cc9ad5dc7aa4167e07` |
| `crates/kairo-ecs-cuda/src/lib.rs` | `3de8c904f7f29d40160f2af42c36b9f3f33cb2586982db0feb3e6b6da62842a3` |
| `crates/kairo-ecs-gpu/src/backends/cuda_backend.rs` | `f2be23e7a4a6e22ef3314bb7fc7f6c99a0b75e74eeadc003e5813647f2c76d86` |
| `crates/kairo-ecs-gpu/tests/backend_initialization.rs` | `bdd791ad0be5384b00a1a768c3b48dff3783c05c04837f587eef50fc6f1c193c` |
| `crates/kairo-ecs-gpu/tests/cuda_native_model.rs` | `423021502f8d4a3e57afa1ad51b9f4484973c716209173559632eb05d088ea64` |
| `crates/kairo-ecs-gpu/tests/cuda_mixed_profile.rs` | `64989df145bbbc8dcbf9131f5eda37137c5cf9ba1e09b81d22d8cd705ca4043c` |
| `production-base.zip` | `a610d0e6707cde45dac72ae76c96cbb8502542cef4bb3f151befc9492ba3bdb2` |
| `production-cuda-source.zip` | `99ed3d245972c7a1c56f403d977b7d36153973c238e4446b3de0676845260374` |
| `host.log` | `eb444c4ce54bb1ca4748699d3d232d2e2a9470ca2666c547ac44530f2826450a` |
| `production-windows.log` | `3c6206a67e0681295e06a4fbbd622d4e3c9177f06e5dc2ed4e3af343ab5028f9` |
| `baseline.log` | `33372e02cb5c9e17329da4e01bec27bddb7ab2442a4845e0da72d295a2e428ec` |
| `candidate.log` | `715d92752f4dca395e4c1399f28d7686f65bd5612085a662dd655382d6debaa3` |
| `local-all.log` | `0f6562313e9079707fdc61052348a53dc8df219529ce3b393c5d53f586c755bb` |
| `local-default.log` | `d6df5a3ecd7e32cd74e01b59ab125c56ab751e2a692f1dc86b2f1e9a8d1e9bf8` |
| `local-clippy.log` | `18cb4e329715f084a78175932d236f4fa064f4981ff9d8d7e043add98cad5442` |
| `local-docs.log` | `489c33a0e712811becc0b859edf3586fed04e749da7ece9d04ec61bb3510fd32` |

## Portable recovery coverage and hardware recheck

The first hosted run passed the workspace tests and reported project line
coverage of 87.42% against the exact base's 87.87%. The resident model now
keeps staging, synchronization, and failure recovery in the same Rust
code exercised by the CUDA driver and a host-side recorded stream. The
CUDA launch and copy calls stay in the driver adapter. The host tests
exercise rejected replacements, dispatch failures, and readback failures.
The facade tests exercise typed errors, native transfer accounting, and
the existing compatibility and host paths.

The RTX 4060 rerun passed the explicit recovery, mixed model, and timing
fixtures, plus 40 ordinary package tests. The final portable assertion
also verifies that an ABM failure invalidates a previously resident
entity buffer. It passed in the 49-test local all-feature package run.
The Nsight trace above belongs to the first measured source.

The Windows rerun used the same workload and 12 paired runs per case.
The 65,536-particle case passed parity and replay, with CPU and CUDA medians
of 4.9567 ms and 2.3125 ms. The median paired speed ratio was 2.149
(95% percentile interval 2.062–2.203). CUDA finished first in all 12 large
cases. CPU finished first in all 24 smaller cases.

| Source or run log | SHA-256 |
| --- | --- |
| `host-coverage-source.zip` applied after the two preceding archives | `fedce6f77c55b1d773be0c3744333fe2bacdab0554b37112493556f2604e7eef` |
| `host-coverage-native-execution.log` | `d51160fcbbdf49102b11d0492110faf8b25ac8a0cc49f8e844710ab08732e4d8` |
| `cuda-host-final-local.lcov` | `5e7c8305c346c75e8fcb0779c74d18ba3c0365247af19ad445791d6f8010e50d` |
