# Retained actual archive-copy provenance oracle

These three files are byte-exact copies of the ignored actual-output oracle
retained with the validator qualification at
`/Volumes/PortableSSD/codex-worktrees/kairos-renovate-python-216-20261005/.artifacts/archive-provenance-validator/`.
They are inputs to a regression test, not a passing provenance statement or a
trusted release attestation.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `provenance.json` | 4,627 | `88423a5c84bdfcfce9c00c0ccdd908686445c3832d968e9fadb250f2f8283c2a` |
| `archive-index.json` | 5,904 | `ff82daf20fe3161040fe87dcfdc1b7607dc3d65e3a176d3713945896c86fce11` |
| `expected-inputs.json` | 1,884 | `bb39f3e431101c2a8b68c830ccf9cc251dcc558f9f494d402c20776978ccf510` |

The source files are preserved unchanged and the test checks these hashes
before validation. The actual statement fails the local profile on timestamp
format and dependency name, URI, and set issues. Separate test copies repair
dependency descriptors or timestamps in memory to isolate the issue classes;
the checked-in actual bytes are never rewritten. The normalized in-memory copy
is only a validator test oracle and does not retroactively qualify the original
statement.
