# Native device and transport measurements

## Mixed particle and DES execution

The Metal fixture initialized the hardware-only Apple M1 Max adapter and
ran one warm-up and twelve alternating CPU/device pairs at each size.
Every pair verified exact additive DES state, particle velocities and device
replay. Particle positions agreed with the CPU within 2e-5. The measured
path includes state upload, every ABM and DES step and final readback.
Setup took 1,261.7 ms on this shared-host run. Cargo 1.99.1 used Homebrew
rustc 1.100.0, identified in the binary. The receipt records the actual
adapter and command. [Raw paired samples](metal.tsv) · [Receipt](receipt.json).

| Particles | Events per batch | Batches | CPU median ms | Metal median ms |
| ---: | ---: | ---: | ---: | ---: |
| 65 | 131 | 4 | 0.008 | 13.409 |
| 4,096 | 16,384 | 16 | 2.755 | 105.930 |
| 65,536 | 65,536 | 8 | 8.706 | 86.433 |

CPU led in all three named Metal profiles. The original Metal release run
also measured CPU ahead at its three sizes. The independently qualified
RTX 4060 run, with the same fixed model sizes and seeds, shows the CUDA
crossover at the largest size. Its 2.261× median paired CPU/CUDA ratio
includes upload and readback. The CUDA record retains setup, host/device
copies, step times, and profiler output, with CPU ahead at both smaller
sizes. [Original Metal profile](../../../../execution/scientific/s11-completion.md) ·
[Native CUDA profile and copy trace](../../../../execution/scientific/s11-cuda-completion.md).

## Local and cross-host recovery

The native mutual-TLS gRPC fixture now has fresh loopback readback for the
uninterrupted reference and termination before and after effect commit.
Both recovery cases matched the reference model digests, full retained
replay state and committed floor. The interval from SIGKILL completion to
the restarted worker's ready report was 1.025 s before commit and 1.028 s
after commit. This interval includes the harness's one-second wait on the
interrupted command thread. The ready report precedes rejoin and final state
verification. The receipt retains all three end-of-scenario boundaries.
[Native fixture](../../../../../docs/distributed/qualify_cap48.py) ·
[Execution receipt](receipt.json).

The retained two-physical-host Time Warp traces contain fifteen paired
kill-to-new-process-ready boundaries, from 0.628 to 2.623 s. This extracts
the next ready report for the same participant within each capture.
The paired values include both local and remote participants. They cover
process recovery within the previously verified two-host fixture.
[Two-host recovery captures](../../../../tracks/48-time-warp-optimistic-rollback-runtime/evidence/external-engine-20261103/receipt.json).

The matched one-host and two-host Mac–NAS ring is a separate transport
measurement. For each sample, subtract the longest worker dispatch in each
of the two ticks from whole-run wall time. The residual includes coordinator
work, synchronization, serialization and communication. It provides a
combined overhead measure for ten samples per configuration.

| Equal-work comparison | One host median residual ms | Two hosts median residual ms |
| --- | ---: | ---: |
| Fixed total work | 1.726 | 48.867 |
| Fixed work per host | 1.495 | 52.854 |

The retained ring profiles matched exact event and state outputs at both
host counts. The wider integrated crash-and-restart comparison remains
part of Track 55. [Physical ring evidence](../../../../tracks/55-end-to-end-weak-strong-scaling-certification/evidence/two-host-ring/README.md) ·
[Same-host optimistic rollback counters](../profiling/time-warp-raw.json).
