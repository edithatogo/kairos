# S13 Native FMI co-simulation

The importer selects the CoSimulation model identifier and GUID or
instantiation token from the extracted XML. It retains the native library
through component destruction. FMI 2 uses synchronous callbacks. FMI 3 uses
the standard step signature and treats partial advancement as a typed error.
Advertised state support enables one in-process checkpoint. Invalid native
state outputs are never recorded as usable handles.
[Compatibility decision and review](../../../docs/adr/0016-native-fmi-co-simulation.md).

## Upstream Reference FMUs

The [Modelica Association v0.0.42 release](https://github.com/modelica/Reference-FMUs/releases/tag/v0.0.42)
provides both BouncingBall archives under BSD-2-Clause. The outer archive
SHA-256 is `b6b4a195b9c0f6bdd3459cd949cbb35fc21ee2049d6dab4cb7a3ba7da63b1e97`.
Its source commit is `75d16f87da2fba3c552c99cd683128a9591c1fa3`.

| FMI | BouncingBall archive SHA-256 |
|---|---|
| 2.0 | `1c7a40ab239c03e4795b230c6fe414a673030d0ce9f26a160301dadca7ff30b7` |
| 3.0 | `9a0093d05b2a029dcfd27237a9eb12e77466803413fb34158c45042617c97de2` |

The independent free-fall and rebound oracle uses initial height 1 m,
gravity 9.81 m/s² and restitution 0.7. Samples are taken every 0.01 s.
Height must match to 0.015 m and velocity to 0.03 m/s. Velocity samples
within 2 ms of the discontinuity are excluded because the native solver
localizes impact on a 1 ms grid. Both trajectories save at 0.4 s and replay
bit for bit across the impact to 0.6 s. They continue after the Rust loader
object is dropped. The target executes both vendor archives and both
Kairos packages of their extracted files. It compares XML, the current
platform's native binary and the complete trajectory bit for bit. The
fixture passed on macOS ARM64 and Windows x86-64 with Rust 1.99.1. The
existing lifecycle tests cover native errors, warning transitions and
ownership across cleanup.

```sh
KAIRO_REFERENCE_FMUS_DIR=/absolute/path/to/verified/reference \
  cargo test --locked -p kairo-ecs-fmi --features fmi2,fmi3 \
    --test maturity_external_fmu bouncing_ball -- --ignored
```

## Independent reader for repackaged FMI 2 and 3

Modelica Association fmusim 0.16.0 validated both original and repackaged
archives and ran their native co-simulation libraries on macOS ARM64. It
sampled height and velocity at 0.01 s intervals from 0 to 0.6 s. The four
61-row CSV files matched byte for byte. The maximum height error against
the analytical oracle was below 0.004361 m. The maximum velocity error
outside the 2 ms impact window was below 0.024622 m/s. Independent ZIP CRC and file
digests matched 18 original members for FMI 2 and 22 for FMI 3.
[Modelica Association fmusim release](https://github.com/modelica/fmusim/releases/tag/v0.16.0).

| Input or output | SHA-256 |
| --- | --- |
| fmusim 0.16.0 macOS ARM64 release archive | `d7d56088cc4a05d1f3fa057ac59e4c1ef31db10cd8dd095c106d9bf623d9fa12` |
| Kairos-repackaged FMI 2 archive | `806019ccf34e0557d2765f6f0bfe51a2a4b5f23c2fae47091cf8ecad4d8a93c8` |
| Kairos-repackaged FMI 3 archive | `7c587f8be566db4afac658e10da9d33eabfa2b97d102630008843d4b9b6822ce` |
| fmusim trajectory CSV | `c941606f0c354bb9579c45d18947df3eba6e4f995b81862ccadb5f8f7025bfe4` |
| Revised Rust native fixture | `ac7ee91dddc1d9dec172e583488097d0af7c83df34a7e6ddf5221d861197f326` |
| macOS fixture log | `63b113087223ac22a83e32264af150fab67276b3358f3a98e628f2e4b0866b42` |
| Windows fixture log | `77f8a58e969843f1db3f8c5f69808f196d0a2b2d725783fea961b3751cb12be8` |

```sh
fmusim validate BouncingBall.fmu
fmusim simulate BouncingBall.fmu --interface-type cs --library-kind native \
  --start-time 0 --stop-time 0.6 --output-interval 0.01 \
  --output-variable h --output-variable v --output-file trajectory.csv
```

## Independent OpenModelica model

The hosted Linux qualification builds a Modelica decay model which applies
`reinit(x, 2)` at 0.5 s. It checks the expected exponential on each side of
that event to 0.012 and checks typed rejection of unavailable FMI 2 state.
The job records the compiler version, model input and emitted FMU hashes.
The public amd64 compiler image uses the digest
`sha256:1fa776fd2f03cdcfc03db6f6ac69888dd48c8eca442034495b178dc73b96dc40`.
[OpenModelica compiler image](https://openmodelica.org/download/docker/).

```sh
KAIRO_OPENMODELICA_FMU=/absolute/path/to/KairosDecay.fmu \
  cargo test --locked -p kairo-ecs-fmi --features fmi2 \
    --test maturity_external_fmu openmodelica -- --ignored
```

The test target and exact input acquisition are wired to the existing
[Rust CI workflow](../../../.github/workflows/ci-core.yml).

## Kairos-authored FMI 2 co-simulation

The standalone scheduled counter builds a native FMU around `Scheduler`.
Modelica Association fmusim 0.16.0 validated the archive and executed its
native library on macOS ARM64. The internal importer and fmusim produced
byte-identical CSVs across 13 samples from 0 to 3 seconds. At ticks 1, 2 and 3,
the outputs were `(47, 2)`, `(472, 3)` and `(4729, 4)` for `(digits, steps)`.
The first step to tick 1 exercised priority ordering for simultaneous events.
The trajectory CSV SHA-256 was
`0844711b4b0479932cc4362ffc8e2e84fac0ba5bbff1b04d2e39f8b4932bf2f5`.
[Build and reproduction commands](../../../docs/fmi-digital-twin/export-guide.md).

OpenModelica OMSimulator's C API also read the FMI 2 archive on Linux x86-64
under Rosetta and reproduced the same output. [FMI 2 qualification](https://github.com/edithatogo/kairos/pull/453).

## Kairos-authored FMI 3 co-simulation

The same scheduler model now emits an FMI 3 archive. On macOS ARM64, Rust
1.99.1 built both versions, and Kairos and fmusim 0.16.0 produced the same
13-row trajectory for both. The archive's output hash matches the FMI 2
trajectory above. The FMI 3.0.2 schema validated the emitted descriptor and
a separate descriptor containing all four supported scalar types.

The native library exposed all 75 symbols in the FMI 3.0.2 headers. A C
client used those headers and loaded the library directly from the archived
bytes to verify token rejection, diagnostic callbacks,
instance independence, reset, scalar output lengths and rejection of an
unknown reference without a partial buffer write. The FMI 3 archive SHA-256
was `e22d4aff4cd34ab9c8bb7f60af1f88b43e70dae86e3d884fb5cd04ff29ca4ea6`.

The remaining external qualification includes OMEdit and the authoring
extensions for clocks, terminals, Model Exchange and Scheduled Execution.
[FMI 3 reproduction commands](../../../docs/fmi-digital-twin/export-guide.md#fmi-3-scalar-co-simulation).
