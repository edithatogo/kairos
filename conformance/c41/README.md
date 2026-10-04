# C4.1 synthetic metric reference packet

This directory contains an experimental synthetic test protocol and independent
reference, not a production metric implementation. The frozen semantics are in
`conductor/design/calibration/c4.1-fixture-contract-v1.md`; the reviewed status,
unit and count clarifications are separate integration inputs and do not alter
that contract's bound input hash.

`fixtures.json` contains 40 cases. `generate_reference.py` recomputes every
rational result with Python `Fraction`, checks serialized expectations, and
separately compares supported equal and weighted cases with SciPy's floating
Wasserstein statistic. Unweighted cases also crosscheck KS. These are numeric
crosschecks only; p-values are discarded. Weighted KS uses only the exact CDF
reference. Each generated record binds the fixtures, generator, lockfile and
wheel provenance by SHA-256.

The isolated interpreter used for generation is external to this source tree:

```sh
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python conformance/c41/generate_reference.py
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python conformance/c41/generate_reference.py --check
/private/tmp/careops-c41-20261005/.artifacts/c41/reference-venv/bin/python -m unittest discover -s conformance/c41 -p 'test_*.py' -v
```

The exact wheel hashes and official PyPI provenance metadata references are in
`requirements-reference.lock` and `wheel-provenance.json`. They are test tools;
no Rust or runtime dependency is introduced.

Comparator mocks in `test_reference.py` only validate comparator behavior. They
are accepted only when a test explicitly passes `self_test_mode=True`. Runtime
candidate checks require `kind=runtime_candidate`, a nonempty producer API
identifier and a 40-digit producer commit. Mock rows are not evidence that
C4.2 produces a metric.

Coverage dispositions under `diagnostic.primary_dispositions` are an
experimental fixture partition, not a public metric count schema. The mutually
exclusive categories are observed, censored, missing, failed, infeasible and
rejected_input. Invalid results have zero used reference/candidate points;
attempted and raw support totals remain in diagnostics. Censor subtype and
overlap diagnostics are non-additive. `unmatched` is zero and marked not
applicable for these unpaired descriptive fixtures.

Supports are already expressed in the declared unit. Matched minute and second
fixtures use the same one-second tick basis: the minute fixture has
`scale_ticks=60` and the second fixture has `scale_ticks=1`. W1 scales by 60 and
KS stays unchanged. Explicit unsigned event-tick cases subtract one common origin, verify exact
offset representability, then divide offsets by `scale_ticks`. Fractional and
signed residual supports remain exact rational values without event-tick
conversion.
