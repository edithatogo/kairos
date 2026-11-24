# Claim validation and file integrity

## Question and prerequisites

Does the published Kairos claim validator reject additional inconsistent
evidence when the file digests supplied to it are internally consistent?
Track 112 accepted the typed validator and receipts. Track 114 accepted
the consumer contract and permitted fixture inputs. The experiment uses
their published records and the bounded native and analytical receipts
in main `077721da0bcb0dcb0bb2f2bc2b8d5cde9778601a`. The planner,
branching and hybrid experiments have separate execution prerequisites.

Workflow Run RO-Crate records the provenance, inputs and outputs of a
workflow. W3C PROV defines constraints over provenance assertions. These
are relevant designs for portable evidence. This experiment compares one
specific check from a file-integrity system with the published Kairos
contract, and uses separate reference evidence for facts about execution.
[Workflow Run RO-Crate](https://doi.org/10.1371/journal.pone.0309210) ·
[W3C PROV constraints](https://www.w3.org/TR/prov-constraints/).

## Frozen corpus and comparison

The 22 cases use public project-owned receipts and source. The original
20 conformance tests informed the design and are outside the scored
corpus. These cases share three receipt lineages. The two installation
platforms share the installation lineage. The science run and the PDES
benchmark are the other two. The unit is one fixed case, with no seed
or inferred population size. The implementation fixes the inputs and
all changes before evaluation.

| Cases | Reference disposition |
| --- | --- |
| V01–V04 | Keep each of the four published receipts in its own scoped capability bundle. |
| V05–V06 | Accept the science and PDES receipts with added JSON whitespace and updated hashes. |
| I01–I04 | Reject stale science and installation receipt hashes, an absent PDES record and a stale Arrow output hash. |
| T01–T02 | Reject a changed C ABI version and a coordinated target edit that disagrees with the retained Rust compiler host. |
| T03–T06 | Reject zero executed cases, a reported failing check, an omitted capability and a conflicting evidence class. |
| T07–T08 | Reject a PDES event-count discrepancy and a shortened timing array. |
| A01–A04 | Reject a false lock digest, false device description, empty Arrow output with a recomputed digest and a false claim about the legacy Sobol sequence when checking against the retained source and execution evidence. |

Expected dispositions come from the original native receipts, the archived
lock and Arrow bytes, and the independent S05 vector and legacy sequence
oracles. The native and PDES mutations recompute the receipt hash except
in the integrity controls. Both compared checks receive the same declared
digests for those files. Integer tokens are preserved exactly.
[S05 source and oracle](../../../execution/scientific/s05-completion.md).

The comparator invokes the platform's `sha256sum -c` on the same declared
paths and digests, including the Arrow output. This is a file-fixity
baseline based on the BagIt payload-manifest check. The published
`validateManifest` function evaluates the same bytes with its typed
contract. [IETF RFC 8493](https://datatracker.ietf.org/doc/html/rfc8493#section-3).

## Decision and reproducibility

The decision requires zero false rejections in V01–V06, both checks
rejecting I01–I04, and the typed check detecting at least two additional
defects across at least two lineages in T01–T08. Record every decision
and disagreement. Report the A cases separately with all accepted
false assertions called out. Counts describe this fixed corpus. Cases
within one lineage are related.

Use Node 26.13.1 and the system SHA-256 command on a clean macOS ARM64
run and a separate Linux x86-64 run. Allow two minutes per runner with
no network or native model execution. Require identical decisions for all
cases. Retain command, source, tool versions, image identity when applicable,
raw results, exit status and hashes. The endpoint is defect detection.
Authenticity of execution and manual review cost require their own
evaluation.

Run `node experiment.mjs <extracted-git-archive>` against a Git archive
of the frozen source above.
