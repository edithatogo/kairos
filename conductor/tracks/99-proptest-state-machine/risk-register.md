# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | Valid transitions/preconditions survive shrinking; injected conservation/replay bug produces reproducible minimal sequence; compare complexity and runtime against plain proptest. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
