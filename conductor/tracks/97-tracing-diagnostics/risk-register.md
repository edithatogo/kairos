# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | Named diagnostic failure becomes explainable; disabled/enabled tracing yields identical simulation outputs; bounded overhead, field redaction, correlated run/worker IDs and backpressure behaviour. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
