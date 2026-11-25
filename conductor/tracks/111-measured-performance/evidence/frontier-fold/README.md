# Conservative frontier calculation

The runtime now visits each advertised bound once to find each destination's
minimum. The fold begins at the requested horizon and preserves the exclusive
safe frontier, including for LPs with no incoming edge. It preserves the exact
queue operations, worker order and counters in the matched workloads.

## Matched result

Both builds used the existing four PHOLD profiles and a fifth 32-LP profile
with eight-tick lookahead and a 75% remote target. Ten fresh process pairs at
each of two event costs alternated execution order, with a warm-up and two timed
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

Run from the repository root with Rust 1.99.1, jq, the locked dependencies
cached and the verified repository-local CPython 3.15.2. The builder compiles
the baseline before applying the frontier patch to the same scratch tree.

```sh
set -euo pipefail
evidence="$PWD/conductor/tracks/111-measured-performance/evidence/frontier-fold"
scratch=$(mktemp -d)
mkdir "$scratch/frontier-source"
git archive "$(jq -er .baseline "$evidence/receipt.json")" | tar -x -C "$scratch/frontier-source"
cp "$evidence/frontier-protocol.json" "$evidence/"*.patch "$scratch"
git -C "$scratch/frontier-source" apply "$scratch/phold-32-lp.patch"
bash "$evidence/build-frontier.sh" "$scratch"
.venv/bin/python -I "$evidence/measure-frontiers.py" "$scratch"
```
