# Conservative frontier calculation

The runtime now visits each advertised bound once to find each destination's
minimum. The fold begins at the requested horizon and preserves the exclusive
safe frontier, including for LPs with no incoming edge. It preserves the exact
queue operations, worker order and counters in the matched workloads.

## Matched result

Both builds used the existing four PHOLD profiles and a fifth 32-LP profile
with eight-tick lookahead and a 75% remote target. Ten fresh process pairs at
each of two event costs alternated build order, with a warm-up and two timed
runs inside each process. The full elapsed times include setup, dispatch and
report extraction. The tape, ordered state and counters matched for all runs.

The 32-LP cheap profile reduced median paired elapsed time by 23.0%. Its
candidate/baseline ratio was 0.770 [0.703, 0.820]. The expensive profile at the
same topology had a ratio of 0.839 [0.836, 0.921]. The worst median regression
in the other eight profiles was 4.5%. The ratios are medians over the ten
within-pair geometric means, with percentile intervals from 10,000 draws.

The experiment was run on a shared Apple M1 Max and met its local adoption
rule of a 20% reduction in the target and at most 5% regression in the other
measured profiles. Dedicated-host certification has its own test matrix.
[Predeclared protocol](frontier-protocol.json) · [All intervals](frontier-summary.json) ·
[Raw paired runs](paired.jsonl.gz) · [Execution receipt](receipt.json).

## Reproduction

Use a new scratch directory, Rust 1.99.1, jq and the verified repository-local
CPython 3.15.2. Extract the receipt's base with `git archive` into
`<scratch>/frontier-source`. Copy `frontier-protocol.json` and both patches
into scratch. Apply `phold-32-lp.patch` inside the extracted tree, then run
`bash build-frontier.sh <scratch>`. The builder runs the baseline before applying
the frontier patch. Run `.venv/bin/python -I measure-frontiers.py <scratch>`
with the repository's interpreter to capture the paired results.
