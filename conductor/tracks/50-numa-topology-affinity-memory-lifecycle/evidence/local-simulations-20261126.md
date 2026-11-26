# Local cluster and NUMA simulation evidence

The 26 November 2026 runs used Kairos source `c538ba3ac47c33436c07c7b5cedf69c0c12d6783`
on one Apple M1 Max Mac with Apple Container 1.7.1. The Kairos binaries used
Rust and Cargo 1.99.1 for Linux x86-64 under Rosetta. The scenario driver
used CPython 3.15.2.

## Two-worker rollback and restart

Two Linux 6.18.56 guests had distinct boot IDs and exchanged gRPC messages
over a private network using mutual TLS. Each worker had its own guest-local
SQLite store. The driver ran the existing CAP-48 reference and durable
admission fault cases with workload seed `48`.
[Qualification fixture](../../../../docs/distributed/qualify_cap48.py)

| Case | Recovery check | Session generations | Global virtual time (GVT) |
| --- | --- | --- | --- |
| `reference` | Uninterrupted run matched the native serial oracle | `1, 1` | `40` ticks |
| `admission-before` | The packet was absent on restart and the retry admitted it once | `2, 2` | `40` ticks |
| `admission-after` | The packet survived restart and the retry returned the retained response without advancing the revision | `2, 2` | `40` ticks |

Both crashes terminated the receiving worker with exit `137`. Both recovery
cases matched the reference's retained state and the native serial oracle.
The two model SHA-256 values were identical across the three cases.

```text
0ec6781ea803ca87e93a4e183824ea61e95b7fa07c8b32984957bfcbabcebb51
b16a9c0352cdff1f383e7c77de0b5f74fe58734526918140858db3b9a57e4591
```

The cluster check exited `0`. Its scope is networked recovery on one
physical host. The test guests, private network and temporary credentials
were removed after the run.

## Emulated NUMA

A temporary ARM64 Linux 6.18.56 guest kernel booted with `CONFIG_NUMA_EMU=y`
and `numa=fake=2`. The successful build disabled ACPI after the earlier
ACPI-enabled build exposed one node. Linux reported nodes `0-1` online and
allowed both for memory allocation. Both nodes listed vCPUs `0-4`.

| Requested node | Linux `MemTotal` | Pages verified on the requested node |
| ---: | ---: | ---: |
| 0 | 1,591,392 kB | 32 |
| 1 | 1,541,036 kB | 32 |

A native ARM64 probe requested each node with `mbind(MPOL_BIND)`, touched
the pages and verified their node counts in `/proc/self/numa_maps`. The two
Kairos `linux_numa` tests passed. Kairos's sysfs discovery agreed with the
kernel's node memory totals, and its thread binding restored the original
CPU mask. The combined guest check exited `0`.
[Linux NUMA tests](../../../../crates/kairo-ecs-core/tests/linux_numa.rs)

The scheduler locality preflight returned exit `77` and identified the
emulated topology as ineligible for locality measurements. CAP-50.5 remains
open for native page placement and matched local and remote timings on a
Linux host with two hardware-backed NUMA nodes.
[Locality probe and run procedure](numa-locality/README.md)

## Saved evidence fingerprints

The original receipts were retained locally and verified against their
SHA-256 manifest. The following digests identify the binaries and outputs
referenced above.

| Artefact | SHA-256 |
| --- | --- |
| CAP-48 worker binary | `8cfd9837d8da0f3c71c3c30bbcc75132d2fc36b2dff7cbcb9557d7aa70b5e956` |
| Three-case cluster receipt | `248eaef5f4dd0a195a77625d6f394e5e6b01201c206261700e4bcea6e43e00a6` |
| Cluster event log | `ce0649587bbacc50daa7c4cba11def85765c77527254e18bb2665c7464b1117b` |
| NUMA guest kernel image | `3f20276be6e2d1cd9450c259b72d013ee922b4d1e9d054a15b7f68db383cb215` |
| NUMA guest kernel configuration | `6e0930a3c70e4a37e1488682d897545bc7eac4bf496ab5ed60a3e7035c65232f` |
| NUMA guest test log | `2703cec19e943079e7c0bfde5f1889e2638a1335cc46d71970b5b6f0150a067f` |
