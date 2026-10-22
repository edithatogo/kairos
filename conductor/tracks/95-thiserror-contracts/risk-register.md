# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | Error variant/code/message parity and source-chain tests; FFI mappings unchanged; no sensitive fields exposed. Count net deleted boilerplate and compile-time impact; leave trivial cases if conversion adds no value. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
