# Handoff — 06 Python Binding 3.10-3.14

## Summary

Python binding work is expected to stay within the interpreter support window captured in this track and to stop before release or cross-language expansion.

## Files changed

`conductor/tracks/06-python-binding-310-314/test-matrix.md`
`conductor/tracks/06-python-binding-310-314/handoff.md`

## Contracts consumed

- Track 01 core types and scheduler contracts.
- Track 12 conformance fixture contracts.
- Track 14 docs workflow only if the binding introduces user-facing docs.

## Contracts changed

- Python binding entrypoints and adapter-level compatibility only.

## Tests added

- Binding tests for the supported Python versions.
- Conformance parity checks when the binding consumes shared fixtures.

## Known risks

- Python version skew across 3.10 through 3.14.
- Native-extension or wheel build drift if package metadata lands before the API stabilizes.
- Cross-language expectations creeping in before the shared fixture contract is finished.

## Integration notes

- Keep this track bounded to the Python surface and binding package shape.
- Do not expand into Track 15 packaging or Track 13 CI policy beyond the local gate commands above.


