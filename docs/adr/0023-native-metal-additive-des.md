# Native Metal additive DES batches

`MetalBackend::initialize_macos_hardware_only` selects an Apple Metal
device and prepares particle and DES pipelines. A successful upload retains
particle state and signed entity values in device buffers. Callers alternate
`run_abm_step` and `run_des_step`, then use `download_state` for explicit
readback. Uploading empty state replaces both buffers. The existing general
constructor supplies the host DES compatibility path.

Each DES event adds a signed 32-bit delta to one entity. The host checks the
whole batch, including entity indices and buffer limits, before submitting
work. One shader invocation per event uses a device atomic addition. Each
value is a modular 32-bit sum. Updates to the same entity yield the same
result across workgroup orders. CPU execution uses the same wrapping
addition in debug and release builds. Dispatch order across batches stays
with the caller, and the Rust scheduler still owns event timing.

An event occupies eight device upload bytes for the entity index and delta.
DES step receipts report the physical upload, workgroup count and zero
downloaded bytes. Initial upload reports both model buffers. The host checks
both shapes before staging. It waits for mapped copies to complete before
publishing the new handles and releases retired buffers after replacement.
The previous handles stay live when an upload fails. A failed native DES
operation fences entity readback and subsequent DES dispatch until a successful
upload. Cloning creates an independent CPU snapshot by reading both buffers.

The hardware fixture checks both signed overflow boundaries, repeated
entity writes across workgroups, invalid input, snapshot isolation and
empty replacement. A separate device validation failure tests reupload
recovery. The mixed fixture interleaves particle and DES steps and compares
positions and entity values against independent expectations.

Run on a selected Apple Metal host with `wgpu-backend` and Rust 1.99.1.
[Device, replay and timing evidence](../../conductor/execution/scientific/s11-completion.md).
