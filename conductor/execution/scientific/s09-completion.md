# S09 Native inference

## ONNX CPU inference

The application supplies an initialized ONNX environment. The native Rust
session selects CPU, checks static float32 ports and keeps the environment
alive across inference calls. The model fixture multiplies state by
`exp(-0.001)` rounded to float32 on every tick. The test uses the independent
analytical value `exp(-tick / 1000)` at each of 1,000 real scheduler dispatches
and allows at most 0.01% relative error. The analytical endpoint is
`0.367879441`. See [ADR 0026](../../../docs/adr/0026-onnx-cpu-inference.md).

The fixture also exercises four concurrent callers, incorrect tensor metadata,
float64 and symbolic ports, malformed models, unsupported operators,
non-finite input and output, use after caller environment drop, and ownership
of returned values after the session drops.

The installed tests ran from the workspace root with Rust 1.99.1 and ONNX
Runtime 1.32.0. Both hosts matched the per-tick oracle and produced the same
final float32 value. The standalone macOS example was installed with Cargo
and reported the same result from outside the source tree.

| Native target | Final state after 1,000 ticks | Installed fixture | Native library SHA-256 |
| --- | ---: | --- | --- |
| macOS ARM64 | 0.367875546 | 1 passed | `fa585fabbea65b823b9a8f7e27d46aee3989d1be7e36b528203d3a1fd613aae7` |
| Linux x86-64 | 0.367875546 | 1 passed | `6c1585d1bbf45b0712ea4a21b705c5fcb4ba04ff2e30f1fbf81a49faf658638f` |

```bash
KAIRO_TEST_ONNX_LIBRARY=/absolute/path/to/libonnxruntime.so.1.32.0 \
  cargo test --locked -p kairo-ecs-ml --features onnx \
  --test maturity_native_inference installed_onnx -- --ignored --nocapture
```

The locked optional edge adds `ort` and `ort-sys` 2.0.0-rc.15. The active
dependency closure reuses the workspace's `libloading` 0.9.0 and
`smallvec` 1.16.4. Rust downloads the binding crates during Cargo dependency
resolution. The test installs the native library separately and checks the
published runtime archive's SHA-256. [Binding release](https://github.com/pykeio/ort/releases/tag/v2.0.0-rc.15) ·
[Runtime distribution](https://pypi.org/project/onnxruntime/1.32.0/).

The macOS release example binary grew from 475,952 bytes to 715,584 bytes
when built with the optional feature. Its separately installed native library
was 35,272,344 bytes. The Linux release probe checked each result over
1,000,000 repeated predictions from a fixed input after 1,000 warm-up calls.
Its resident size was 31,720 KiB at the end of warm-up and at each subsequent
100,000-call sample.

Full Cargo Deny passed advisories, licences and sources. Its bans check
reported the same path-only Arrow development dependency at the base and
candidate. That dependency belongs to the staged native checkpoint fixture.

The model bytes are generated from the checked-in text protobuf using
`protoc` 36.2 and the ONNX 1.22.0 schema. The decay model SHA-256 is
`76fdee247aaf72417fe77a2199d879c11c965ec267923639f3d29be3a412ed72`.
[Model source](../../../crates/kairo-ecs-ml/tests/data/unit-decay-v1.textproto) ·
[ONNX schema](https://github.com/onnx/onnx/blob/v1.22.0/onnx/onnx.proto).

## Rust affine surrogate

`AffineSurrogate` reuses `NeuralSystem` and tensor validation. It binds model
identity and version to caller-supplied coefficients and a declared input
domain. It rejects incompatible shapes, non-finite coefficients, overflowing
output and out-of-domain inputs.

Check a non-identity matrix against hand-calculated values. Use independently computed exponential decay over the bounded training domain as a held-out surrogate test with a maximum absolute error of 0.025. Preserve the scheduler hook and its fallback observability.
