# Risk register

Severity: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Added complexity without benefit | High | Value gate and explicit reject/defer outcome |
| Changed contract or false acceptance | High | A seeded concurrency bug is found; repaired model passes bounded interleavings; native tests agree; state-space/time budgets documented and unmodelled operations explicit. |
| Ownership or migration cycle | High | Exact claims and dependency-sequencing register |
