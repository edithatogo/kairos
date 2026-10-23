# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | Concurrent directory creation/isolation; cleanup on normal/error paths; intentional retained diagnostics; same-filesystem publication, failure before/after persist, no-clobber and old-output preservation. End-to-end process-kill/disk-full/manifest recovery remains mandatory in Polars and is not certified by this supporting track. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
