#!/usr/bin/env python3
"""Root-owned Mac/Colab Track49 gRPC qualification; Python3.15.2 only.

No provider allocation or authentication recovery occurs here. This controller
uses one prequalified pinned SSH route, actual mTLS/model workers and typed
Tonic/Arrow IPC. Model assertions are imported from the native MPI oracle.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import pathlib
import queue
import signal
import subprocess
import sys
import tempfile
import threading
import time
import uuid

if sys.version_info[:3] != (3, 15, 2):
    raise SystemExit("Use the verified repository-local Python3.15.2")
ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import qualify_track49 as oracle
from qualify_cap48 import pki_script
from track49_remote import RemoteHost, make_source_packet, source_path


def checked(args, **kw):
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          timeout=kw.pop("timeout", 90), **kw)


def protobuf_fields(raw):
    """Bounded fixture inspection only; the maintained Rust SDK admits snapshots."""
    assert len(raw) <= 65536
    fields, offset = {}, 0

    def integer():
        nonlocal offset
        value = 0
        for shift in range(0, 70, 7):
            assert offset < len(raw), "truncated fixture protobuf"
            byte = raw[offset]
            offset += 1
            value |= (byte & 127) << shift
            if byte < 128:
                return value
        raise AssertionError("fixture protobuf integer bound")

    while offset < len(raw):
        tag = integer()
        number, wire = tag >> 3, tag & 7
        assert number > 0 and wire in (0, 2)
        if wire == 0:
            value = integer()
        else:
            length = integer()
            assert offset + length <= len(raw)
            value = raw[offset:offset + length]
            offset += length
        fields.setdefault(number, []).append(value)
    return fields


def inspect_snapshot(hex_frame):
    raw = bytes.fromhex(hex_frame)
    outer = protobuf_fields(raw)
    assert outer[1] == [2] and len(outer[2]) == 1
    snapshot = protobuf_fields(outer[2][0])
    assert snapshot[1] == [11] and snapshot[2] == [7]
    assert snapshot.get(3, [0]) == [0] and snapshot[4] == [1]
    assert len(snapshot[6]) == 1
    component = protobuf_fields(snapshot[6][0])
    assert component[1] == [b"track49.entity.v1"] and component[2] == [13]
    entity = json.loads(component[3][0])
    pending = [protobuf_fields(x) for x in snapshot[7]]
    assert [(int.from_bytes(p[3][0], "big"), p[4][0], bool(p.get(6, [0])[0])) for p in pending] == [
        (45, 3, False), (45, 4, False), (47, 6, False), (47, 6, True), (50, 9, False)]
    assert all(p.get(1, [0]) == [0] and p.get(2, [0]) == [0] for p in pending)
    assert all((p.get(5, [0])[0] == 0) == bool(p.get(6, [0])[0]) for p in pending)
    assert entity["generation"] == 7 and entity["component_generation"] == 13
    assert len(entity["pending"]) == 5 and entity["ownership"] == 1
    return {"migration_id": snapshot[5][0].decode(), "entity": entity,
            "pending_count": len(pending), "wire_sha256": hashlib.sha256(raw).hexdigest()}


class GrpcNode:
    def __init__(self, group, participant, settings, attempt):
        self.group, self.owner, self.participant = group, group.owner, participant
        self.settings = pathlib.Path(settings)
        self.remote = participant == 1
        self.responses = queue.Queue()
        self.lock = threading.Lock()
        self.expected_failure = False
        self._stopped = False
        self.pid = None
        stem = f"{group.name}-node{participant}-{attempt}-{uuid.uuid4().hex[:8]}"
        self.raw = (self.owner.output / (stem + ".stdout.log")).open("w")
        self.stderr = (self.owner.output / (stem + ".stderr.log")).open("w")
        if self.remote:
            self.pidfile = f"launch-{attempt}-{uuid.uuid4().hex}.pid"
            self.proc = self.owner.host.popen_owned(
                [self.owner.remote_binary, str(self.settings)], self.settings.parent,
                self.pidfile, stderr=self.stderr)
        else:
            self.proc = subprocess.Popen([str(self.owner.binary), str(self.settings)],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
                text=True, bufsize=1, start_new_session=True)
        self.owner.nodes.append(self)  # Track ownership before any readiness assertion.
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        ready = self.responses.get(timeout=40)
        assert isinstance(ready, dict) and set(ready) == {"ready", "pid", "listen", "sync_listen", "generation"}, ready
        assert ready["ready"] is True and type(ready["pid"]) is int and ready["pid"] > 1, ready
        assert type(ready["generation"]) is int and ready["generation"] >= 0, ready
        assert ready["listen"] == f"127.0.0.1:{43148+participant}", ready
        assert ready["sync_listen"] == f"127.0.0.1:{43150+participant}", ready
        self.pid = ready["pid"]
        self.owner.event("grpc_worker_ready", name=group.name, participant=participant,
                         remote=self.remote, pid=self.pid, generation=ready["generation"],
                         listen=ready.get("listen"), sync_listen=ready["sync_listen"])

    def _read(self):
        for line in self.proc.stdout:
            self.raw.write(line)
            self.raw.flush()
            if "STORE_BOUNDARY" in line:
                self.group.boundaries.put(line.strip())
                continue
            try:
                self.responses.put(json.loads(line))
            except json.JSONDecodeError:
                self.owner.event("native_stdout", participant=self.participant, line=line[:512])
        self.responses.put({"process_exited": self.proc.poll()})

    def _rpc(self, op, **kw):
        with self.lock:
            assert self.proc.poll() is None, "owned worker exited before command"
            self.proc.stdin.write(json.dumps(dict(op=op, **kw), separators=(",", ":")) + "\n")
            self.proc.stdin.flush()
            response = self.responses.get(timeout=45)
        assert isinstance(response, dict) and type(response.get("ok")) is bool, response
        if response["ok"]:
            assert set(response) == {"ok", "result"} and isinstance(response["result"], dict), response
        else:
            assert set(response) == {"ok", "error"}, response
            if op.startswith("sync_"):
                error = response["error"]
                assert isinstance(error, dict) and set(error) == {"kind", "code", "message", "details", "metadata"}, response
                assert all(isinstance(error[k], str) for k in ["kind", "code", "message", "metadata"]), response
                assert isinstance(error["details"], list) and all(type(x) is int and 0 <= x <= 255 for x in error["details"]), response
            else:
                assert isinstance(response["error"], str), response
        self.owner.event("grpc_command", name=self.group.name, participant=self.participant,
                         operation=op, ok=response["ok"], error=response.get("error"))
        return response

    def call(self, op, expect=True, **kw):
        if op == "send" and kw["destination"] == self.participant:
            # Fixed self-delivery has no network peer. Historical captured roots
            # use explicit recovery ingress after a complete successor join.
            state = self.call("snapshot")
            ingress = "recover" if oracle.decode(kw["packet"])["generation"] < state["generation"] else "receive"
            response = self._rpc(ingress, packet=kw["packet"])
            if response.get("ok") and kw.get("accept", True):
                for reply in response["result"]["replies"]:
                    accepted = self._rpc("receive", packet=reply)
                    if not accepted.get("ok"):
                        response = accepted
                        break
        else:
            response = self._rpc(op, **kw)
        if expect:
            assert response.get("ok"), response
        return response["result"] if response.get("ok") else response

    def kill(self):
        self.expected_failure = True
        if self.remote:
            if self.pid is None:
                self.pid = self.owner.host.read_pidfile(self.settings.parent, self.pidfile)
            result = self.owner.host.signal_owned(self.pid, "track49_node", str(self.settings))
            assert result.get("signal_sent") == signal.SIGKILL, result
        else:
            if self.pid is None:
                self.pid = self.proc.pid
            actual = checked(["ps", "-p", str(self.pid), "-o", "command="]).stdout
            assert "track49_node" in actual and str(self.settings) in actual
            self.proc.kill()
        code = self.proc.wait(timeout=20)
        assert code != 0, "SIGKILL cannot qualify as clean shutdown"
        self.owner.event("grpc_worker_sigkill", name=self.group.name, participant=self.participant,
                         pid=self.pid, transport_exit=code)

    def stop(self):
        if self._stopped:
            return
        if self.proc.poll() is None:
            self.proc.stdin.write('{"op":"stop"}\n')
            self.proc.stdin.flush()
            try:
                self.proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                # Cleanup is explicit and guarded, never a successful result.
                self.kill()
                raise AssertionError("owned worker did not stop within the terminal bound")
        self.reader.join(timeout=2)
        assert not self.reader.is_alive(), "native stdout reader did not become terminal"
        self.raw.close()
        self.stderr.close()
        if self.remote:
            if self.pid is None:
                self.pid = self.owner.host.read_pidfile(self.settings.parent, self.pidfile)
            status = self.owner.host.owned_status(self.pid, "track49_node", str(self.settings))
            assert not status.get("exists") or status.get("state") == "Z", status
        else:
            if self.pid is None:
                self.pid = self.proc.pid
            actual = subprocess.run(["ps", "-p", str(self.pid), "-o", "command="], capture_output=True, text=True)
            assert not ("track49_node" in actual.stdout and str(self.settings) in actual.stdout)
        self._stopped = True
        if not self.expected_failure:
            assert self.proc.returncode == 0, "unexpected native failure is not acceptance"


class GPeerGroup:
    def __init__(self, owner, name, settings, attempt=0):
        self.owner, self.name, self.n = owner, name, 2
        self.settings, self.attempt = settings, attempt
        self.boundaries = queue.Queue()
        self.members = []
        self.failed = False
        self.sync_attempted = False
        self._stopped = False
        owner.groups.append(self)  # Constructor failures remain cleanup-owned.
        for participant in range(2):
            node = GrpcNode(self, participant, settings[participant], attempt)
            self.members.append(node)

    def kill_rank(self, rank):
        assert rank == 1, "qualification targets the real remote receiver"
        self.failed = True
        self.members[rank].kill()
        # Both engines reopen standby, preserving the same two private stores.
        for node in self.members:
            if node.participant != rank:
                node.stop()

    def stop(self):
        if self._stopped:
            return
        error = None
        if self.name in self.owner.passed_cases and not self.failed and not self.sync_attempted:
            self.sync_attempted = True  # Failed checks are never automatically retried.
            try:
                self.owner.qualify_sync(self)
            except Exception as exc:
                error = exc
        for node in self.members:
            try:
                node.stop()
            except Exception as exc:
                if error is None:
                    error = exc
                self.owner.event("cleanup_error", name=self.name, error=str(exc))
        self._stopped = all(node._stopped for node in self.members)
        if error is not None:
            raise error


class Owner:
    def __init__(self, args):
        self.args = args
        self.output = pathlib.Path(args.output).resolve()
        self.output.mkdir(mode=0o700, parents=True, exist_ok=True)
        self.events = (self.output / "events.jsonl").open("a")
        self.run_id = uuid.uuid4().hex
        self.base = pathlib.Path(tempfile.mkdtemp(prefix="track49-grpc-"))
        self.binary = pathlib.Path(args.binary).resolve()
        self.remote_repo = pathlib.PurePosixPath(args.remote_repo)
        self.remote_base = self.remote_repo.parent / ("grpc-qualification-" + self.run_id)
        self.groups, self.nodes, self.passed_cases, self.sync_results = [], [], set(), []
        self.host = RemoteHost(args.ssh_args, self.remote_repo, self.output / "remote")
        self.host_started = False
        self.blackholes = []
        self.source = {n: hashlib.sha256((ROOT / n).read_bytes()).hexdigest() for n in self.source_paths()}

    def source_paths(self):
        selection = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo"]
        selection += ["crates/" + n for n in ["kairo-ecs-pdes", "kairo-ecs-grpc", "kairo-ecs-mpi",
                      "kairo-ecs-arrow", "kairo-ecs-types", "kairo-ecs-core", "kairo-ecs-rng", "kairo-ecs-fs-guard"]]
        selection += ["docs/distributed/track49-fixtures", "docs/distributed/qualify_track49.py",
                      "docs/distributed/qualify_cap48.py", "docs/distributed/track49_remote.py"]
        paths = checked(["git", "ls-files", *selection], cwd=ROOT).stdout.splitlines()
        paths.append("docs/distributed/qualify_track49_grpc.py")
        result = []
        for name in sorted(set(paths)):
            # Non-source benchmark assets cannot affect this example build.
            if pathlib.PurePosixPath(name).suffix not in {".rs", ".toml", ".lock", ".proto", ".py", ".json", ".md"}:
                continue
            source_path(name)
            result.append(name)
        return result

    def event(self, event, **kw):
        if event == "scenario_pass":
            self.passed_cases.add(kw["name"])
        self.events.write(json.dumps(dict(event=event, monotonic=time.monotonic(), **kw)) + "\n")
        self.events.flush()

    def freeze_check(self):
        assert self.source == {n: hashlib.sha256((ROOT / n).read_bytes()).hexdigest() for n in self.source}
        assert self.binary_hash == hashlib.sha256(self.binary.read_bytes()).hexdigest()
        if self.host_started:
            script = f"""import pathlib,json,hashlib
repo=pathlib.Path({str(self.remote_repo)!r});source=json.loads({json.dumps(self.source)!r})
assert all(hashlib.sha256((repo/n).read_bytes()).hexdigest()==h for n,h in source.items())
p=pathlib.Path({self.remote_binary!r});assert hashlib.sha256(p.read_bytes()).hexdigest()=={self.remote_binary_hash!r}
print('TRACK49_GRPC_REMOTE_FROZEN')
"""
            assert self.host.remote_python(script).strip() == "TRACK49_GRPC_REMOTE_FROZEN"

    def setup(self):
        environment = dict(os.environ, CARGO_TARGET_DIR=str(self.binary.parents[2]), CARGO_PROFILE_DEV_DEBUG="0",
            CARGO_PROFILE_DEV_OPT_LEVEL="1", CARGO_BUILD_JOBS="2", RUSTUP_TOOLCHAIN="1.99.1",
            RUSTC=checked(["which", "rustc"]).stdout.strip(), RUSTDOC=checked(["which", "rustdoc"]).stdout.strip())
        command = ["cargo", "build", "--locked", "--message-format=json", "--config=profile.dev.opt-level=1",
                   "--config=profile.dev.debug-assertions=true", "--config=profile.dev.overflow-checks=true",
                   "--config=profile.dev.package.kairo-ecs-pdes.opt-level=3",
                   "--config=profile.dev.package.kairo-ecs-grpc.opt-level=3",
                   "-p", "kairo-ecs-grpc", "--example", "track49_node", "--features", "external-fault-injection,telemetry-ipc"]
        build = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True, timeout=900)
        (self.output / "local-build.jsonl").write_text(build.stdout)
        (self.output / "local-build.log").write_text(build.stderr)
        assert build.returncode == 0, "fresh gRPC worker build failed"
        rows = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
        artifacts = [r for r in rows if r.get("reason") == "compiler-artifact" and r.get("target", {}).get("name") == "track49_node" and r.get("executable")]
        assert artifacts and pathlib.Path(artifacts[-1]["executable"]).resolve() == self.binary
        engines = [r for r in rows if r.get("reason") == "compiler-artifact" and r.get("target", {}).get("name") == "kairo_ecs_pdes"]
        self.profiles = {"local_worker": artifacts[-1]["profile"], "local_engine": engines[-1]["profile"]}
        assert all(p["opt_level"] == "3" and p["debug_assertions"] and p["overflow_checks"] for p in self.profiles.values())
        assert {"external-fault-injection", "telemetry-ipc"} <= set(artifacts[-1]["features"])
        self.binary_hash = hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.description = json.loads(checked([str(self.binary), "--describe"]).stdout)
        assert self.source == {n: hashlib.sha256((ROOT / n).read_bytes()).hexdigest() for n in self.source}
        (self.output / "local-rust.txt").write_text(checked([environment["RUSTC"], "-vV"]).stdout)
        self.host.start()
        self.host_started = True
        packet = make_source_packet(ROOT, self.source)
        self.host.upload_source(packet, self.source)
        result = self.host.cargo(command[1:], ["track49_node"], log_name="grpc-checked-build-" + self.run_id + ".log", source_hashes=self.source, timeout=900)
        assert result["exit"] == 0 and not result["timed_out"], result
        artifact = result["executables"]["track49_node"]
        assert artifact["target_kind"] == ["example"]
        raw_remote_log = (self.host.output / ("grpc-checked-build-" + self.run_id + ".log")).read_text()
        remote_rows = [json.loads(line) for line in raw_remote_log.splitlines() if line.startswith("{")]
        remote_artifacts = [r for r in remote_rows if r.get("reason") == "compiler-artifact" and r.get("target", {}).get("name") == "track49_node" and r.get("executable")]
        remote_engines = [r for r in remote_rows if r.get("reason") == "compiler-artifact" and r.get("target", {}).get("name") == "kairo_ecs_pdes"]
        assert remote_artifacts and remote_engines
        self.profiles.update(remote_worker=remote_artifacts[-1]["profile"], remote_engine=remote_engines[-1]["profile"])
        assert all(p["opt_level"] == "3" and p["debug_assertions"] and p["overflow_checks"] for p in self.profiles.values())
        assert set(remote_artifacts[-1]["features"]) == set(artifacts[-1]["features"])
        self.remote_binary, self.remote_binary_hash = artifact["path"], artifact["sha256"]
        (self.output / "remote-build.json").write_text(json.dumps(result, indent=2))
        self.public = []
        for participant in range(2):
            pki = self.remote_base / "pki" if participant else self.base / "pki"
            script = pki_script(pki, f"node{participant}")
            raw = self.host.remote_python(script) if participant else checked([str(ROOT / ".venv/bin/python"), "-"], input=script).stdout
            self.public.append(json.loads(raw))
        self.event("grpc_owner", source_base=checked(["git", "rev-parse", "HEAD"], cwd=ROOT).stdout.strip(),
                   source=self.source, local_binary=self.binary_hash, remote_binary=self.remote_binary_hash,
                   local_base=str(self.base), remote_base=str(self.remote_base))
        self.freeze_check()

    def group(self, name, n):
        assert n == 2
        config = {"simulation": list(hashlib.sha256((self.run_id + ":" + name).encode()).digest()[:16]),
                  "model": self.description["model"],
                  "members": [{"participant": p, "public_key": self.description["public_keys"][p],
                               "certificate_hash": self.public[p]["hash"], "lps": [p]} for p in range(2)],
                  "topology": [[0, [1]], [1, [0]]]}
        (self.output / (name + "-configuration.json")).write_text(json.dumps(config, indent=2))
        settings_paths = []
        for p in range(2):
            base = (self.remote_base if p else self.base) / name / f"node{p}"
            pki = self.remote_base / "pki" if p else self.base / "pki"
            settings = {"configuration": config, "participant": p, "store": str(base / "store"),
                "listen": f"127.0.0.1:{43148 + p}", "certificate": str(pki / "leaf.pem"),
                "private_key": str(pki / "leaf.pk8"), "client_ca": str(base / "client-ca.pem"),
                "peers": [{"participant": 1 - p, "endpoint": f"https://127.0.0.1:{43149 - p}",
                           "domain": f"node{1-p}", "ca": str(base / "peer-ca.pem"),
                           "certificate_hash": self.public[1-p]["hash"]}],
                "sync_listen": f"127.0.0.1:{43150 + p}", "sync_peers": [{"participant": 1-p,
                    "endpoint": f"http://127.0.0.1:{43151-p}"}], "sync_timeout_ms": 1000}
            script = f"""import pathlib,json,os
base=pathlib.Path({str(base)!r});(base/'store').mkdir(mode=0o700,parents=True,exist_ok=False)
(base/'client-ca.pem').write_text({(self.public[0]['ca']+self.public[1]['ca'])!r})
(base/'peer-ca.pem').write_text({self.public[1-p]['ca']!r})
p=base/'settings.json';p.write_text({json.dumps(settings)!r});p.chmod(0o600)
"""
            if p:
                self.host.remote_python(script)
            else:
                checked([str(ROOT / ".venv/bin/python"), "-"], input=script)
            settings_paths.append(base / "settings.json")
        group = GPeerGroup(self, name, settings_paths)
        return group

    def restart(self, group):
        replacement = GPeerGroup(self, group.name, group.settings, group.attempt + 1)
        return replacement

    @staticmethod
    def failure(response, kind, code):
        assert response.get("ok") is False and isinstance(response.get("error"), dict), response
        error = response["error"]
        assert error["kind"] == kind and error["code"] == code, error
        assert isinstance(error["details"], list) and isinstance(error["metadata"], str)
        return error

    def qualify_sync(self, group):
        a, b = group.members
        before = [oracle.model(a), oracle.model(b)]
        checkpoint = json.loads((self.output / (group.name + "-node0-checkpoint.json")).read_text())
        wires = {}
        for outgoing in checkpoint["outgoing"]:
            message = outgoing["message"]
            if message["anti"]:
                continue
            data = json.loads(bytes(message["payload"]))
            if data.get("op") == "Transfer":
                wire = data["data"]["wire_snapshot"]
                wires[wire] = inspect_snapshot(wire)
        assert len(wires) == 2, "retain both conflicting incarnation snapshots"
        old = next(w for w, info in wires.items() if info["entity"]["value"] == 5)
        new = next(w for w, info in wires.items() if info["entity"]["value"] == 105)
        assert wires[old]["migration_id"] == wires[new]["migration_id"]
        ack = a.call("sync_snapshot_send", destination=1, wire_snapshot=new)
        assert ack["applied"] is False
        assert a.call("sync_snapshot_send", destination=1, wire_snapshot=new)["applied"] is False
        permanent_snapshot = self.failure(a.call("sync_snapshot_send", destination=1, wire_snapshot=old, expect=False), "Permanent", "AlreadyExists")
        snapshots = b.call("sync_snapshot_drain")["snapshots"]
        assert snapshots == [{"version": 2, "wire_snapshot": new}]
        assert b.call("sync_snapshot_drain")["snapshots"] == []
        tick = 1 << 100
        rows0 = [{"event_index": 100, "event_generation": 2, "entity_index": 11, "entity_generation": 7,
                  "tick": f"{tick:032x}", "priority": -2, "sequence": 3, "event_kind": "transfer", "status": "dispatched", "payload_ref": "synthetic:entity11"},
                 {"event_index": 101, "event_generation": 3, "entity_index": None, "entity_generation": None,
                  "tick": f"{tick+1:032x}", "priority": 0, "sequence": 4, "event_kind": "cancel", "status": "cancelled", "payload_ref": None}]
        rows1 = [{"event_index": 100, "event_generation": 2, "entity_index": 11, "entity_generation": 7,
                  "tick": f"{tick:032x}", "priority": -1, "sequence": 3, "event_kind": "remote", "status": "skipped", "payload_ref": "synthetic:remote"}]
        run = "track49-" + group.name
        def make(node, source, rows, batch_id):
            return node.call("sync_telemetry_make", batch_id=batch_id, run_id=run, source=source,
                             tick_start=f"{tick:032x}", tick_end=f"{tick+1:032x}", rows=rows)["batch"]
        batch0, batch1 = make(a, 0, rows0, "source0"), make(b, 1, rows1, "source1")
        assert a.call("sync_telemetry_send", destination=1, batches=[batch0])["accepted_batches"] == 1
        assert a.call("sync_telemetry_send", destination=1, batches=[batch0])["accepted_batches"] == 0
        assert b.call("sync_telemetry_send", destination=0, batches=[batch1])["accepted_batches"] == 1
        changed = copy.deepcopy(rows0)
        changed[0]["payload_ref"] = "synthetic:conflict"
        conflict = make(a, 0, changed, "source0")
        permanent_telemetry = self.failure(a.call("sync_telemetry_send", destination=1, batches=[conflict], expect=False), "Permanent", "AlreadyExists")
        received0 = b.call("sync_telemetry_drain")["batches"]
        received1 = a.call("sync_telemetry_drain")["batches"]
        assert received0 == [batch0] and received1 == [batch1]
        merged = a.call("sync_telemetry_merge", batches=received1 + received0 + received0, expected_sources=[0, 1, 2])
        assert merged["missing_sources"] == [2] and merged["row_sources"] == [0, 1, 0]
        expected = [dict(rows0[0], run_id=run), dict(rows1[0], run_id=run), dict(rows0[1], run_id=run)]
        assert merged["rows"] == expected and merged["arrow_ipc_payload"]
        reverse = b.call("sync_telemetry_merge", batches=received0 + received1, expected_sources=[0, 1])
        assert reverse["rows"] == expected and reverse["missing_sources"] == []
        assert reverse["row_sources"] == merged["row_sources"]
        assert reverse["arrow_ipc_payload"] == merged["arrow_ipc_payload"]
        merge_conflict = self.failure(a.call("sync_telemetry_merge", batches=[batch0, conflict], expected_sources=[0, 1], expect=False), "Permanent", "AlreadyExists")
        event_conflict_batch = make(a, 0, changed, "source0-event-conflict")
        event_conflict = self.failure(a.call("sync_telemetry_merge", batches=[batch0, event_conflict_batch], expected_sources=[0, 1], expect=False), "Permanent", "AlreadyExists")
        assert [oracle.model(a), oracle.model(b)] == before, "queue admission is not model application"
        b.stop()
        transient = self.failure(a.call("sync_snapshot_send", destination=1, wire_snapshot=new, expect=False), "Transient", "Unavailable")
        sentinel = pathlib.Path(group.settings[1]).parent / "blackhole.settings"
        code = """import json,os,socket,time,sys
role='track49-blackhole';s=socket.socket();s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1);s.bind(('127.0.0.1',43151));s.listen(4)
print(json.dumps({'ready':True,'pid':os.getpid(),'role':role}),flush=True)
c,_=s.accept();time.sleep(20)
"""
        error_log = (self.output / (group.name + "-blackhole.stderr.log")).open("w")
        pidfile = "blackhole-" + uuid.uuid4().hex + ".pid"
        proc = self.host.popen_owned([str(self.remote_repo / ".venv/bin/python"), "-c", code, str(sentinel)],
            sentinel.parent, pidfile, stderr=error_log)
        owned = {"proc": proc, "pid": None, "pidfile": pidfile, "settings": str(sentinel), "log": error_log, "terminal": False}
        self.blackholes.append(owned)  # Register before readiness parsing can fail.
        try:
            ready_queue = queue.Queue()
            threading.Thread(target=lambda: ready_queue.put(proc.stdout.readline()), daemon=True).start()
            ready = json.loads(ready_queue.get(timeout=20))
            assert set(ready) == {"ready", "pid", "role"} and ready["ready"] is True and ready["role"] == "track49-blackhole"
            assert type(ready["pid"]) is int and ready["pid"] > 1
            owned["pid"] = ready["pid"]
            partition = self.failure(a.call("sync_snapshot_send", destination=1, wire_snapshot=new, expect=False), "PartitionSuspected", "DeadlineExceeded")
        finally:
            self.stop_blackhole(owned)
        assert oracle.model(a) == before[0]
        result = {"name": group.name, "snapshot": wires[new], "snapshot_queue_applied": False,
            "telemetry_rows": expected, "row_sources": merged["row_sources"], "missing_sources": [2],
            "merged_ipc_sha256": hashlib.sha256(bytes(merged["arrow_ipc_payload"])).hexdigest(),
            "failures": {"snapshot_conflict": permanent_snapshot, "telemetry_conflict": permanent_telemetry,
                         "merge_conflict": merge_conflict, "event_conflict": event_conflict,
                         "unavailable": transient, "partition_suspected": partition}}
        (self.output / (group.name + "-typed-sync.json")).write_text(json.dumps(result, indent=2))
        (self.output / (group.name + "-typed-telemetry.json")).write_text(json.dumps({"source0": batch0, "source1": batch1, "merged": merged}, indent=2))
        self.sync_results.append(result)
        self.event("typed_sync_pass", name=group.name, snapshot_applied=False, telemetry_rows=3,
                   failure_kinds=["Permanent", "Transient", "PartitionSuspected"])

    def stop_blackhole(self, owned):
        if owned["terminal"]:
            return
        if owned["pid"] is None:
            owned["pid"] = self.host.read_pidfile(pathlib.PurePosixPath(owned["settings"]).parent, owned["pidfile"])
        status = self.host.owned_status(owned["pid"], "track49-blackhole", owned["settings"])
        if status.get("exists") and status.get("state") != "Z":
            sent = self.host.signal_owned(owned["pid"], "track49-blackhole", owned["settings"])
            assert sent.get("signal_sent") == signal.SIGKILL, sent
        assert owned["proc"].wait(timeout=20) != 0
        owned["log"].close()
        owned["terminal"] = True

    def close(self):
        active_error = sys.exc_info()[0] is not None
        errors = []
        for group in self.groups:
            try:
                group.stop()
            except Exception as exc:
                errors.append(exc)
                self.event("cleanup_error", name=group.name, error=str(exc))
        for node in self.nodes:
            try:
                node.stop()
            except Exception as exc:
                errors.append(exc)
                self.event("cleanup_error", scope="partial_node", error=str(exc))
        for owned in self.blackholes:
            try:
                self.stop_blackhole(owned)
            except Exception as exc:
                errors.append(exc)
                self.event("cleanup_error", scope="blackhole", error=str(exc))
        if self.host_started:
            try:
                terminal = self.host.terminal_statuses()
                self.event("remote_terminal", **terminal)
                assert not terminal["active_owned_case_pids"], terminal
                self.host.close()
            except Exception as exc:
                errors.append(exc)
                self.event("cleanup_error", scope="remote", error=str(exc))
                self.host.close(require_terminal=False)
        self.events.close()
        if errors and not active_error:
            raise errors[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "artifacts/track49/build-opt1/debug/examples/track49_node"))
    parser.add_argument("--ssh-args", required=True)
    parser.add_argument("--remote-repo", default="/content/kairos-track49-full/kairos")
    parser.add_argument("--output", required=True)
    parser.add_argument("--cases", choices=["reference", "all"], default="all")
    args = parser.parse_args()
    owner = Owner(args)
    try:
        owner.setup()
        results = [oracle.scenario(owner, "reference", 2)]
        if args.cases == "all":
            for after in [False, True]:
                result = oracle.scenario(owner, "model-commit-" + ("after" if after else "before"), 2, after)
                assert result["semantics"] == results[0]["semantics"], "recovery changed retained model/identity/replay semantics"
                results.append(result)
        owner.freeze_check()
        assert len(owner.sync_results) == len(results)
        receipt = {"passed": len(results), "failed": 0, "two_hosts": True, "source": owner.source,
            "binary_sha256": owner.binary_hash, "remote_binary_sha256": owner.remote_binary_hash,
            "cases": [{k: v for k, v in r.items() if k != "semantics"} for r in results],
            "typed_sync": owner.sync_results, "runtime": "real mTLS external engine, two-host SQL SIGKILL/reopen joins, typed Tonic full snapshots and maintained Polars Arrow IPC",
            "profiles": owner.profiles,
            "python": {"version": sys.version, "executable": sys.executable, "prefix": sys.prefix}}
    finally:
        owner.close()
    receipt["owned_process_cleanup_complete"] = True
    (owner.output / "receipt.json").write_text(json.dumps(receipt, indent=2))
    print(json.dumps({"TRACK49_GRPC_PASSED": len(results), "two_hosts": True}), flush=True)


if __name__ == "__main__":
    main()
