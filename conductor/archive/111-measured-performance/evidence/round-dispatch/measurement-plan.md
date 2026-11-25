Compare the same 2,048-input tape at seed 472026, with 4,096 dispatches and
four LPs, at 0 and 8,192 mixes per event on both conservative executors.
Keep the event tape, ordered final state and runtime report equal. Use ten
fresh process pairs per case, one warm-up and two opposite-order measured
repetitions per process. Compare median pair ratios and bootstrap intervals
from 10,000 resamples with seed 111. Record setup, dispatch, total and RSS.

Retain the candidate only with at least 20% lower pooled zero-work total
time and no greater than 5% higher total time in the other matched cases.
The threshold uses median paired ratios. The unmodified serial scheduler
is a noise control. Measure allocations in a separate instrumented build.
