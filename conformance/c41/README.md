# C4.1 synthetic metric reference packet

This directory contains an experimental synthetic test protocol and independent
reference, not a production metric implementation. The frozen semantics are in
`conductor/design/calibration/c4.1-fixture-contract-v1.md`. The reviewed status,
unit and count clarifications are separate integration inputs and do not alter
that contract's bound input hash.

`fixtures.json` contains 37 cases. `generate_reference.py` recomputes every
rational result with Python `Fraction`, checks the serialized expected values,
and separately compares supported equal and weighted cases with SciPy's
floating Wasserstein statistic. Unweighted cases also crosscheck KS. These
SciPy values are crosschecks only; p-values are discarded. Weighted KS uses
only the exact CDF reference.
Each generated record binds the fixtures, generator, lockfile and wheel
provenance by SHA-256. Generation timestamps belong in execution receipts.

The isolated interpreter used for generation is external to this source tree:

```sh
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python conformance/c41/generate_reference.py
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python conformance/c41/generate_reference.py --check
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python -m unittest discover -s conformance/c41 -p 'test_*.py' -v
```

The exact wheel hashes and official PyPI provenance metadata references are
recorded in `requirements-reference.lock` and `wheel-provenance.json`. They are
test tooling only; no Rust or runtime dependency is introduced. Mock candidate
rows in `test_reference.py` only validate comparator behavior. They are not
evidence that C4.2 produces a metric.

Coverage counts use mutually exclusive primary dispositions under
`diagnostic.primary_dispositions`: observed, censored, missing, failed and
infeasible. Censor subtype and overlapping diagnostics are non-additive.
`unmatched` is zero and marked not applicable for these unpaired descriptive
fixtures. Supports are already expressed in the declared unit. The duration
pair shows W1 scaling by 60 between minute and second supports with KS
unchanged. `origin` appears only on explicit unsigned event-tick cases; offsets
are divided by `scale_ticks` after exact common-origin subtraction and
representability checks. Fractional and signed residual supports remain exact
rational values.
