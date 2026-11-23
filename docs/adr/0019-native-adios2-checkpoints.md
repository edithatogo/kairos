# Native ADIOS2 BP5 checkpoints

The Rust `adios2` feature uses serial BP5 on Linux and macOS. The optional
`kairo-ecs-adios2` crate links to the installed ADIOS2 2.12 C library and
owns the C handles and synchronous buffer transfers. Set `ADIOS2_LIB_DIR`
to the native library directory and include it in the loader's search path.
Set a nonempty `HOME` for ADIOS2 user configuration. The adapter returns an
error before calling the library if the variable is missing or empty.
Qualification uses ADIOS2 2.12.1. The main Arrow crate remains safe Rust.

The public adapter and receipt retain their Rust shape. The destination is
a BP5 directory, and a successful receipt records `contract_only = false`.
The legacy reader and writer remain available through
`CheckpointManifest::{read_contract_file,write_contract_file}`. Use a new BP5
destination when migrating an existing regular file. Unsupported targets
return the existing adapter-disabled error.

## Storage and integrity

The schema marker `kairo-ecs-adios2-bp5-checkpoint-v1` identifies one BP5 step.
Each event and block field is a primitive rank-one global array. Time is two
unsigned 64-bit columns. Priority is signed 32-bit. Separate presence columns
preserve null entities and absent versus empty payloads. The UTF-8 arena and
per-row byte counts preserve embedded NULs. The six counts give the lengths
of the events, blocks, text arena, ID, run ID and schema fingerprint. Arrays
whose declared length is zero are omitted.

A native reader checks the direct BP5 directory members, step, type, rank and
shape. Its allocations are bounded by the uncompressed data file's physical
size. The shared record codec verifies the legacy checksum and a versioned
SHA-256 digest over the complete logical rows and metadata. Use a trusted
artifact and validate its origin separately when authentication matters.

## Publication and recovery

Write and read back the complete generation in a private sibling staging
directory, then sync the files and directory. Linux and macOS provide the
atomic no-clobber rename and directory exchange used for publication. Existing
destinations must be valid Kairos BP5 checkpoints. A replacement preserves
ordinary permission bits and checks owner and group. ACL and special mode-bit
preservation require separate qualification.

The previous generation remains at the staging path until directory syncs
complete. After a post-exchange sync or cleanup error, inspect the reported
destination and staging path before retrying. Owners use a trusted, stable
parent directory and coordinate publication across processes. A BP5 reader
opens multiple members. External readers that overlap a replacement can retry
after publication, or use immutable generation names. A process-local lock
covers Rust adapter reads and writes through publication.

The conformance cases check the wide limbs and signed priority with `bpls`,
and exercise empty arrays, nullability, corruption, path rejection and
replacement. Live scheduler and parallel filesystem qualification have their
own Track 51 gates. [ADIOS2 release](https://github.com/ornladios/ADIOS2/releases/tag/v2.12.1).
