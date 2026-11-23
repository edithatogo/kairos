# Native CUDA particle and additive DES batches

Enable `cuda-backend` in `kairo-ecs-gpu` and use
`CudaBackend::initialize_hardware_only` to require a CUDA device for both
particle and additive DES steps. The driver supplies the device name, total
memory and compute capability. The selected context loads both kernels.
Initialization reports distinguish a disabled build from a failed driver or
device initialization.

`upload_state` validates the model shapes, stages both buffers and waits for
the copies before publishing their handles. A failed replacement preserves
the previous handles. Particles and signed entity values stay on the device
across dispatched steps. `download_state` completes the copies before
returning independent host vectors. An unsuccessful kernel submission
requires a new upload before the hardware-only path can use the model again.

The particle step uses the same low 32 seed bits and particle index hash
as the CPU reference. Each DES invocation adds one signed 32-bit delta.
The sums wrap modulo 2³², so repeated writes to an entity have deterministic
results across workgroups. Validate every entity identity before submitting
the batch. The caller retains scheduler and timestamp ownership.

Transfer receipts count explicit buffers. The initial upload accounts for
both model buffers. The DES batch uses an eight-byte entity index and delta
pair per event. The compatibility constructor retains its host DES path and
reports the actual eager particle readback when it selects CUDA.

The Windows qualification exercises the public Rust path on an NVIDIA
RTX 4060, confirms seeded motion against the scalar reference and compares
the same workloads on CPU and CUDA. The fixture timings measure the local
device, host and driver combination. Broader deployment and scaling claims
use the corresponding platform and live evidence gates.
[Native and timing evidence](../../conductor/execution/scientific/s11-cuda-completion.md).
