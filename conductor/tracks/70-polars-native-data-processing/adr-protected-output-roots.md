# Enforced protected output roots

Status: implementation candidate; platform and package gates remain required.
Human direction: 29 October 2026, proceed with enforced protected output roots.

## Decision and scope

Both `DurableRunPublicationWriter::publish` and `RunOutput::{create,resume}`
require an existing output root owned by the current OS identity and protected
from mutation by other OS users. Root permissions and ACLs are checked on the
opened handle before a run is created or resumed. A symlink as the output root
is rejected. No caller directory is silently chmodded or created recursively.

New runs are created exclusively and opened relative to the held root. New Unix
run directories request `0700` and staging files request `0600`. The resulting
run is verified as private before data is written. Windows uses its native DACL
profile; inherited file-only access grants must also pass the check. Native ACL
code is isolated in `kairo-ecs-fs-guard`; `kairo-ecs-arrow` keeps its existing
`forbid(unsafe_code)` policy. The helper's README defines accepted local
filesystem and ACL profiles and explicit unsupported behavior.

All publication staging, no-clobber promotion, cleanup and directory syncing use
the held run directory. Staging keeps its open file handle through publication
and verifies the final file identity. Staging names have process/call suffixes;
exclusive creation handles collisions, including stale names after process-ID
reuse. The suffix is a uniqueness device, not an access-control mechanism.

`RunOutput` retains the root and run handles through writes and completion and
rechecks their binding and permissions. Resume enumerates and reads through the
held run. Completed readers hold one run handle and refuse symlinked part files.
Existing final files are never replaced. Parts remain staged and synced before
atomic no-replace publication; the completed manifest is published last.

## Security boundary

The guarantee starts at the validated opened root. Untrusted OS users cannot
replace entries beneath that root or alter new private output files under the
accepted local permission model. Directory-relative operations preserve that
binding if an outer ancestor is renamed. Receipt path checks detect observed
replacement, but do not promise that pathnames remain stable after return.

Processes sharing the publisher's OS identity, administrators, hostile filesystem
servers and trusted users changing permissions after checks are outside this
boundary. A dedicated protected root is required. Shared writable output roots
are rejected; there is no automatic permissive fallback or security exception.
Checksums continue to detect content divergence rather than authenticate data
against an actor authorized to replace a manifest and its parts.

## Compatibility and recovery

Successful output schemas, ordering, part names, manifest fields and the
checkpoint-to-trace boundary remain unchanged. `RunOutput` retains its
`.staging-` prefix and existing interrupted-run semantics. Abandoned staging files
are ignored, preserved, and do not block later resume because each new write uses
an exclusive unique staging name. Old non-private interrupted directories are
rejected for resume; choosing or preparing a private recovery root is explicit.
Completed historical runs remain readable if their manifest and parts validate.

Callers must now prepare a protected root before publication. Missing roots,
unsafe modes, rejected ACLs and unsupported permission models fail explicitly
before output data is written. The smoke-only legacy atomic-file utility has a
separate caller-trusted-path contract and is not a protected run publisher.

## Acceptance and remaining gates

Focused tests must preserve normal publication, empty runs, completed/interrupted
no-clobber behavior, bounded batches, disk-error visibility, checkpoint resume,
and IPC/Parquet interoperability. Rejection tests cover shared-writable roots,
macOS ACL grants behind private mode bits, Windows file-only inherited grants,
symlink roots/parts, path replacement, existing final parts and stale staging
names. Unsupported capability is an error, not a blanket test skip.

macOS, Linux and Windows runtime results, independent review disposition,
clean-package evidence, and the existing security audit are separate gates.
The existing zstd packaging blocker is not waived by this change. No Conductor
phase or repository status is advanced merely because the focused tests pass.

