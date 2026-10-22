# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Run only for a concrete FMI consumer; if rust-fmi replaces this whole path, use its facilities and avoid duplicate direct dependencies or extracting twice. |
| Changed semantics hidden by updated goldens | High | Independent compressed/stored/ZIP64 fixtures where supported, malformed/truncated CRC, traversal, symlink/duplicate entries, expansion limit, partial extraction cleanup and FMU roundtrip. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |
