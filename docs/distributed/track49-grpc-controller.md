# Track49 two-host gRPC qualification controller

`qualify_track49_grpc.py` is the root-owned synthetic Mac/Colab controller. The
worker implementation lane produces source and local adapter/syntax checks only;
those checks are not native acceptance. Provider/session/authentication ownership
stays with root. The controller allocates or stops no Colab VM.

```sh
.venv/bin/python docs/distributed/qualify_track49_grpc.py \
  --binary artifacts/track49/build-opt1/debug/examples/track49_node \
  --ssh-args PRIVATE_QUALIFIED_SSH_ARGS_JSON \
  --remote-repo /content/kairos-track49-full/kairos \
  --output PRIVATE_ATTEMPT_DIRECTORY --cases all
```

The private SSH route is already qualified and strictly pinned. `RemoteHost`
owns one muxed connection and loopback forwarding for actual mTLS43148/43149
and typed Tonic43150/43151. The controller builds dependencies with dev
opt-level1/debug0 and overrides the PDES engine and gRPC worker packages to
opt-level3. Debug assertions and overflow checks remain enabled and are asserted
from both hosts' actual engine/worker compiler artifacts. The first opt-level1
two-host attempt timed out adopting the common cut on the CPU-limited peer;
both workers terminated and that uncertain operation was not retried. This
qualified-profile exercise retains the same RPC deadlines and model oracles.
The controller checks actual Cargo compiler-artifact executable paths,
features and optimization profiles, and retains measured local/remote binary
hashes and exact logs. An explicit public-source packet includes the native
model, runtime, transport, Arrow dependencies, controller and helper inputs;
source/binary hashes are checked after build and after qualification. No stale
binary can qualify after a failed build.

PKI private keys are generated on their own hosts. Only public CA/certificates,
public pin hashes and frozen public source are exchanged. Private settings,
native stores, provider tokens and private bindings never enter the source packet.
The selected real repository Python3.15.2 and Rust1.99.1 are retained in receipts.

## Shared model oracle and process ownership

The controller imports the existing `qualify_track49.scenario` unchanged. It
adapts its group/node interface to two actual JSON-controlled Rust workers.
Reference plus before/after receiver SQL-commit SIGKILL cases preserve the same
entity/component generations, pending positive/anti metadata, RNG, logical keys,
all applied Work+ACK descendants, delayed rollback and blocked replacement,
serial parity, common floor80 and strict-before-floor rejection assertions.

A receiver failure kills the exact remote PID after verifying executable marker
and exact settings argv. The surviving worker is stopped; both existing private
stores reopen in standby and require a complete successor join. Old committed
ACKs must remain byte-identical with no new receiver revision. No command is
automatically retried and no MPI live-rank recovery is implied.

Nodes and groups are registered before readiness validation, including partial
constructors. JSON response envelopes and structured typed errors are checked
exactly. EOF/unexpected response shapes fail. Repeated stop calls are idempotent;
unexpected process exits are not accepted as clean termination. Historical
self-delivery uses explicit recovery ingress only after a new joined generation.

## Typed full-state and Arrow IPC checks

Before each successful scenario's workers stop, the controller uses actual
Tonic RPCs to qualify the separate typed data queues:

- Send the model's canonical v2 full migration snapshot, repeat it, reject a
  conflicting same-identity incarnation, and drain exactly one byte-identical
  snapshot. Its pending queue retains all five metadata entries, including the
  same-tick sequence ordering and positive/anti pair; entity generation7 and
  component generation13 remain exact. The Python protobuf reader only inspects
  the fixture; maintained Rust SDK code performs admission and serialization.
- Assert the transport ACK says `applied=false`, and neither model changes from
  typed queue acceptance or telemetry activity.
- Produce real schema-v2 IPC with maintained Polars on both hosts, send batches
  both directions, suppress an exact duplicate, reject conflicting batch and
  event identity, and deterministically merge full-width ticks above2^100,
  negative priorities, null entity fields, statuses and explicit source IDs.
  Missing source2 is reported. Opposite input orders produce identical rows,
  source alignment and IPC bytes.
- Preserve actual `AlreadyExists/Permanent` conflict status, then stop the remote
  peer and require `Unavailable/Transient`. A guarded remote TCP blackhole on
  that same owned typed port forces the existing connection/RPC deadline and
  requires `DeadlineExceeded/PartitionSuspected`. This is suspicion, never proof
  of death or authority/GVT advancement. Deadlines and error policies are unchanged.

The blackhole is registered before readiness parsing and always enters guarded
cleanup, using its PID file if readiness fails. There is no process-name kill.
The controller records native stdout, exact source/configuration/binary hashes,
checkpoints, typed rows/IPC and full failure status metadata. It publishes the
success receipt only after all owned native processes are terminal and the owned
SSH master closes. VM termination remains root's separate provider operation.

## Local-only source qualification

The implementation lane runs repo-local Python3.15.2 `py_compile` and synthetic
adapter checks for exact JSON envelopes, typed failure shape, byte-preserving ACK
results, explicit historical self-recovery, idempotent stop and source-set bounds.
These perform no native worker/SSH/Colab operations (source selection uses
read-only Git) and cannot establish any
runtime, network, durable replay, telemetry or provider acceptance.
