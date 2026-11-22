# ADR 0018 Native HDF5 checkpoints

## Status

This decision covers the optional `kairo-ecs-arrow` HDF5 record checkpoint API.

## Decision

The existing `Hdf5CheckpointAdapter` writes the
`kairo-ecs-hdf5-checkpoint-v1` root schema. The `records` dataset is a
contiguous compound table. It carries two unsigned time limbs, generational
identities, signed priorities, sequences, status codes and presence flags.
The `text` dataset stores the concatenated UTF-8 bytes addressed by each
row's kind and payload lengths. The `blocks` dataset preserves the block
manifest. Status codes 0 through 3 mean dispatched, cancelled, skipped and
error in that order.

The SHA-256 dataset covers logical metadata, ordered records and their complete
values. Keep the legacy checksum for the public manifest type. The reader
checks required field types and extents, local backing and the digest. Only
direct local datasets with contiguous storage are accepted. Authenticate a
producer with evidence from outside the checkpoint file.

The same-directory staging helper closes the HDF5 file before file sync and
rename. Validation and staging failures preserve the current generation.
A parent-directory sync error occurs after rename and reports uncertain
durability. Read the destination to determine which generation is present.

## Compatibility

The feature links to an installed HDF5 library through `hdf5-metno` 0.15.1.
The build qualifies the official HDF5 2.2.0 distribution. The required library,
headers and loader path must be available on hosts building with `hdf5`.
Explicitly choose its root with `HDF5_DIR` when package discovery is ambiguous.

The native adapter requires UTF-8 checkpoint paths. It rejects historical
text files. Load them with `CheckpointManifest::read_contract_file` and pass
the recovered HDF5-labelled manifest to the adapter to migrate it. This file
format carries event records and block metadata. Saving and restoring the
live scheduler queue is a Track 51 follow-up.

## Review

Independent review covered one-byte corruptions, logical digest coverage,
presence flags, the size checks used before reading and the staging boundary.
It identified two ways a successful write could produce an unreadable file.
The writer now rejects invalid path encoding before staging and creates empty
datasets with the same contiguous layout as nonempty ones. Tests cover empty
checkpoints and legacy files, and the vendor reader examines the actual output.
[Track 51 evidence](../../conductor/execution/scientific/s14-completion.md).
