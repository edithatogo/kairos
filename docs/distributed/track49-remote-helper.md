# Track49 owned remote helper

`track49_remote.py` is a standard-library Python3.15.2 helper for the root native
qualification controller. It allocates no Colab session, changes no credentials,
and performs no model/SQL operations itself. The caller supplies its existing
qualified private SSH argument-list JSON and owns all remote/native activity.

## Interface

```python
host = RemoteHost(private_ssh_args_json, remote_repo, private_output_dir).start()
packet = make_source_packet(local_repo, frozen_public_source_paths)
hashes = host.upload_source(packet, frozen_public_source_paths)
build = host.cargo(
    ["build", "--locked", "-p", "kairo-ecs-mpi", "--example", "track49_mpi"],
    ["track49_mpi"], log_name="mpi-build-attempt1.log", source_hashes=hashes,
)
assert build["exit"] == 0
binary = build["executables"]["track49_mpi"]  # path, SHA256, kind, package ID
```

The provided SSH list ends in its destination, retains explicit
`StrictHostKeyChecking=yes`, and has no existing ControlMaster/forwarding scope.
The helper starts one short `/tmp/t49-*/ctl` ControlMaster and preserves its
account/host pins and original proxy route. Mux commands retain those pins but
replace fallback proxy routing with `false`: loss of the owned master fails
closed instead of creating a second provider connection. Forwarding is limited
to loopback43149/43148 for mTLS and43151/43150 for Tonic, in the directions
specified by the controller. Startup verifies the actual remote interpreter is
3.15.2 with the selected repository `.venv` prefix. It never prints supplied SSH
arguments, provider tokens, scripts or private command output.

Public source packets use the exact schema
`{relative_path: {"sha256": lower_hex, "base64": canonical_base64}}`. A frozen
caller-selected allowlist is mandatory. Paths are canonical relative text-source
paths; traversal, absolute paths, credential/native binding names, keys, SQL/DB
files, private/artifact/environment directories and symlink escapes reject.
Bounds:256 files,4 MiB/file,16 MiB aggregate. This guard supplements the caller's
public-source selection; it is not a general secret-content classifier. Existing
native stores, bindings, sessions and private keys must never enter a packet.

`cargo` uses the selected root-local Rustup/Cargo homes, explicit Rust1.99.1
compiler/rustdoc, locked Cargo arguments, two jobs and debug0 dev/test profiles.
It checks source hashes before/after, reads actual JSON `compiler-artifact`
executables and their SHA256 (not guessed paths), and retains/fetches the exact
raw log. Logs are created exclusively per attempt; old failed logs are not
overwritten. Build timeout terminates only the new owned Cargo process group.
Remote transport timeout remains an unknown remote outcome requiring readback;
no command is automatically retried. Raw Cargo readback is bounded to64 MiB.

## Owned process control

```python
proc = host.popen_owned(command_list, case_path, "launch1.pid")
# proc is the local SSH Popen with stdin/stdout pipes for the JSON controller.
pid = host.read_pidfile(case_path, "launch1.pid")
state = host.owned_status(pid, "track49_mpi", exact_settings_or_case_argument)
host.signal_owned(pid, "track49_mpi", exact_settings_or_case_argument)
```

Cases must be inside the owned provider root (`remote_repo.parent`), outside the
source repository. The remote wrapper establishes its own session when requested,
records its actual native-manager PID in a new mode600/exclusive PID file, then
execs the supplied command with preserved pipes. It handles an SSH session that
is already a session leader without a failing second `setsid`. Local SSH PID is
never treated as remote PID. Use a distinct PID file per launch; an existing
file is a recovery journal, not permission to relaunch.

`signal_owned` checks exact `/proc/PID/cmdline` for the executable marker and an
**exact argv item** equal to the case/settings path before signaling. Optional
`group=True` additionally requires the owned PID is its process-group leader.
There is no process-name kill, blind PID kill or automatic native retry.
`terminal_statuses` reads exact registered-case argv matches and ignores zombies.
The caller must include its case/settings path as an argv item for these guards;
MPI daemon cleanup and provider teardown still belong to the root controller.

`close()` checks for active registered-case processes and refuses to abandon
live native jobs. The caller must terminate them explicitly through its guarded
scope. `close(require_terminal=False)` is an explicit transport-only shutdown
for unavailable/failed routes and proves no native termination. Only the owned
ControlMaster/local transport is closed. Frozen receipts or root-owned terminal
readback establish actual native/provider cleanup.

## Local helper qualification

The worker used actual repo-local Python3.15.2 for `py_compile` and a local-only
synthetic guard test. It checked generated remote Python syntax, path/hash and
allowlist rejections, fail-closed mux command construction, PID-wrapper syntax
and base64 raw-log readback with its trailing newline. No SSH, Colab, native
process, model, migration or runtime proof was produced by those helper tests.
