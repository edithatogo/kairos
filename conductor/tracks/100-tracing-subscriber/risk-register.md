# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | Filtering/redaction, duplicate initialization handling, bounded logging/backpressure, stdout machine-output separation, enabled/disabled outcome equality and measured cost. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
