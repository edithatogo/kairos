# Julia binding

Track 08 owns this package. Julia currently provides metadata and event-log data
conversion; it has no qualified simulation runtime. Kairos execution must run
in Rust. `ffi_status().configured` is false and `self_check()` reports
`metadata_only`. File discovery does not claim a loaded engine or Arrow C Data
support.

The existing `EventLogRecord` / `EventLogBatch` types and `ordered_events`
helper operate on data supplied by the caller. Sorting an existing log is data
processing, not an alternative scheduler. The Schema v2 smoke-byte codec
copies records into escaped tabular text with decimal integer limbs; it is not
Arrow IPC, native Arrow buffer handoff, or evidence of Rust engine execution.
Entity ID limbs must be null together and malformed records are rejected.

Fixture bridge helpers read ready/planned fixture metadata. A fixture marked
ready upstream does not prove that Julia executed it. Native engine and
populated Arrow support require a Rust FFI adapter, owned lifetimes, artifact
packaging and actual native conformance tests before release qualification.

Validation from this directory:

```bash
julia --project=. -e 'using Pkg; Pkg.test()'
julia --project=. -e 'include("test/test_arrow.jl")'
```
