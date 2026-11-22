# S13 Native FMI co-simulation

The importer selects the CoSimulation model identifier and GUID or
instantiation token from the extracted XML. It retains the native library
through component destruction. FMI 2 uses synchronous callbacks. FMI 3 uses
the standard step signature and treats partial advancement as a typed error.
Advertised state support enables one in-process checkpoint. Invalid native
state outputs are never recorded as usable handles.

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
object is dropped. The target executes both binary FMUs on macOS ARM64
with Rust 1.99.1. The existing lifecycle tests cover native errors, warning
transitions and ownership across cleanup.

```sh
KAIRO_REFERENCE_FMUS_DIR=/absolute/path/to/verified/reference \
  cargo test --locked -p kairo-ecs-fmi --features fmi2,fmi3 \
    --test maturity_external_fmu bouncing_ball -- --ignored
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
Additional exporters, Model Exchange, Scheduled Execution and an export
roundtrip have separate qualification work.
