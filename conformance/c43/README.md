# C4.3 independent Arrow readback

`readback.py` is a qualification reader for the private calibration output
adapter. It reads the actual Rust-written `residual.ipc_file`,
`residual.ipc_stream`, `residual.parquet`, and corresponding `metric.*` files
from one directory. `residual.json` and `metric.json` are the producer's actual
canonical C0 logical record arrays. They are the readback comparison input;
the Python reader does not construct runtime outputs from expected values.

The reader freezes schemas independently from the Rust writer and enforces field
order, Arrow type, nullability, global and field metadata, C0 JSON Schema,
canonical ordering and duplicate-key rejection. It checks exact-u128 byte
encoding, residual sign/magnitude, metric null/status/count/unit/window rules,
finite values, and optional run/event links. It verifies the unchanged
`event_log_v1` schema digest. The script is not imported by the Rust runtime.

Run the standalone self-test with the existing C1 qualification interpreter
(CPython 3.14.8, PyArrow 25.0.1):

```sh
/private/tmp/kairos-c1-ci-python-qualify-20261004/.artifacts/c1-ci-python-qualification/venv-attempt2/bin/python conformance/c43/readback.py --self-test
```

For producer output:

```sh
/private/tmp/kairos-c1-ci-python-qualify-20261004/.artifacts/c1-ci-python-qualification/venv-attempt2/bin/python conformance/c43/readback.py "$C43_OUTPUT_DIR"
```

The logical reader evaluates the bounded JSON Schema keyword subset used by
the unchanged C0 residual and metric definitions. It fails closed if those
definitions add unsupported keywords. The PyArrow venv remains the only Python
package dependency. Rust feature and MSRV qualification is recorded by the
native producer owner.
