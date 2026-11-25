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

## Pinned toolchain repeat

Cargo 1.99.1 had selected Homebrew rustc 1.100.0 for both builds in the first
capture. The executable paths embedded by the linked Rust standard library
identify that compiler revision. A second capture explicitly selected rustc
and rustdoc 1.99.1 before building both the baseline and the candidate.

Across another ten process pairs at each event cost, the pinned build
preserved exact input, state and counter parity. The cheap 32-LP profile
reduced median paired total time by 23.4%. The candidate/baseline ratio
was 0.766 [0.733, 0.835]. At 8,192 mixes its ratio was
0.886 [0.839, 1.028]. The largest comparator median increase was 4.7%.
Both captures met the preregistered local point-estimate and parity rule.
[Pinned receipt](pinned-receipt.json) · [All pinned intervals](pinned-summary.json) ·
[Pinned raw pairs](pinned-paired.jsonl.gz).

## Reproduction

Run from the repository root with Cargo and Rust 1.99.1, jq, the locked dependencies
cached and the verified repository-local CPython 3.15.2. The builder compiles
the baseline before applying the frontier patch to the same scratch tree.

```sh
set -euo pipefail
evidence="$PWD/conductor/tracks/111-measured-performance/evidence/frontier-fold"
scratch=$(mktemp -d)
mkdir "$scratch/frontier-source"
git archive --format=tar --output "$scratch/source.tar" "$(jq -er .baseline "$evidence/receipt.json")"
tar -xf "$scratch/source.tar" -C "$scratch/frontier-source"
cp "$evidence/frontier-protocol.json" "$evidence/"*.patch "$scratch"
git -C "$scratch/frontier-source" apply "$scratch/phold-32-lp.patch"
bash "$evidence/build-frontier.sh" "$scratch"
.venv/bin/python -I "$evidence/measure-frontiers.py" "$scratch"
```

To reproduce the first capture, set `RUSTC` and `RUSTDOC` to the verified
Homebrew 1.100.0 binaries before calling the builder. The default selects the
pinned compiler explicitly.
