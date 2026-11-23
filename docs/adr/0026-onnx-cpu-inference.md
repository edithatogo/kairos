# ADR 0026 Opt-in ONNX CPU inference

The `onnx` feature of `kairo-ecs-ml` uses `ort` 2.0.0-rc.15 with its dynamic
loader and ONNX C API 31. The existing `OrtSession` descriptor binds to the
application's initialized environment through `with_cpu_runtime`. The
unit-decay example executes the model and reports its analytical reference.
The feature selects the small binding dependency set and retains the existing
Tract non-adoption decision. [Binding release](https://github.com/pykeio/ort/releases/tag/v2.0.0-rc.15) ·
[Dependency decision](../../conductor/tracks/89-tract-robustness/adr-tract-inference-evaluation.md).

## Rust compatibility

Construction from bytes or a file continues to validate descriptor metadata.
Binding consumes the descriptor and creates a CPU session from in-memory model
bytes. The test matrix uses ONNX Runtime 1.32.0 on Linux x86-64 and macOS ARM64.
A session accepts one fixed-shape float32 input and produces one fixed-shape
float32 output. The existing Rust tensor checks reject invalid input and
non-finite results. The output owns its values after the next run or session drop.

The application owns process-global library selection and environment options.
Kairos accepts that initialized environment explicitly. The session retains
it, selects CPU, and uses one intra-operation thread. Model clones share the
session and serialize calls. The standalone example and installed test disable
runtime telemetry when building their environment.

## Qualification

The application supplies trusted native library and model files. Build-time dependency
fetches cover the Rust binding. CI verifies the publisher's ONNX Runtime archive
digest before extracting the native library, and runs the executable fixture
under workspace coverage.

The fixture evaluates actual scheduler dispatches through `InferenceTickHook`.
It checks analytical decay, concurrent calls, invalid libraries and models,
unsupported operators, incompatible metadata, non-finite values, and use of
an owned output after the session drops. The result applies to the model
and ports exercised by the fixture.
[Published runtime](https://pypi.org/project/onnxruntime/1.32.0/) ·
[Fixture source](../../crates/kairo-ecs-ml/tests/data/unit-decay-v1.textproto).
