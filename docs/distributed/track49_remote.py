#!/usr/bin/env python3
"""Owned Track49 SSH mux, public-source transfer and guarded native processes.

This module never allocates a provider session or changes authentication. The
calling controller owns its already-qualified SSH argument file and native jobs.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
import pathlib
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time
import uuid
from collections.abc import Iterable, Mapping, Sequence

if sys.version_info[:3] != (3, 15, 2):
    raise RuntimeError("Track49 remote helper requires repository-local Python3.15.2")

MAX_SOURCE_FILE = 4 * 1024 * 1024
MAX_SOURCE_TOTAL = 16 * 1024 * 1024
MAX_SOURCE_FILES = 256
MAX_LOG_BYTES = 64 * 1024 * 1024
_SOURCE_SUFFIXES = {".rs", ".toml", ".lock", ".proto", ".py", ".json", ".md"}
_FORBIDDEN_PARTS = {".git", ".ssh", ".venv", ".codex", ".agents", ".secrets", ".private",
                    ".config", "private", "artifacts", "target", "__pycache__"}
_FORBIDDEN_NAMES = {"sessions.json", "settings.json", "credentials.json", "secrets.json",
                    "auth.json", "tokens.json", "native-bindings.json"}


def source_path(name: str) -> str:
    """Accept only canonical repository-relative text-source paths."""
    if not isinstance(name, str) or not name or len(name.encode()) > 512:
        raise ValueError("invalid source path")
    p = pathlib.PurePosixPath(name)
    if (p.is_absolute() or str(p) != name or "\\" in name or ":" in name
            or any(ord(c) < 32 for c in name) or ".." in p.parts
            or any(part in _FORBIDDEN_PARTS for part in p.parts)
            or p.name.lower() in _FORBIDDEN_NAMES or p.suffix not in _SOURCE_SUFFIXES):
        raise ValueError("noncanonical or non-source path")
    return name


def validate_packet(packet: Mapping, allowed_paths: Iterable[str]) -> dict:
    """Validate an explicit frozen allowlist, hashes, base64 and resource bounds."""
    allowed = {source_path(n) for n in allowed_paths}
    if not isinstance(packet, Mapping) or not 0 < len(packet) <= MAX_SOURCE_FILES:
        raise ValueError("invalid source packet count")
    result, total = {}, 0
    for name, item in packet.items():
        source_path(name)
        if name not in allowed or not isinstance(item, Mapping) or set(item) != {"sha256", "base64"}:
            raise ValueError("source outside frozen allowlist or invalid packet schema")
        sha, encoded = item["sha256"], item["base64"]
        if not isinstance(sha, str) or not re.fullmatch("[0-9a-f]{64}", sha):
            raise ValueError("invalid source SHA256")
        if not isinstance(encoded, str) or len(encoded) > ((MAX_SOURCE_FILE + 2) // 3) * 4:
            raise ValueError("source file too large")
        raw = base64.b64decode(encoded, validate=True)
        if (len(raw) > MAX_SOURCE_FILE or hashlib.sha256(raw).hexdigest() != sha
                or base64.b64encode(raw).decode("ascii") != encoded):
            raise ValueError("source hash, size or canonical base64 mismatch")
        raw.decode("utf-8")
        total += len(raw)
        if total > MAX_SOURCE_TOTAL:
            raise ValueError("source packet too large")
        result[name] = {"sha256": sha, "base64": encoded}
    return result


def make_source_packet(root: pathlib.Path, paths: Iterable[str]) -> dict:
    root = root.resolve(strict=True)
    packet = {}
    for name in paths:
        source_path(name)
        p = root / name
        if p.is_symlink() or not p.resolve(strict=True).is_relative_to(root) or not p.is_file():
            raise ValueError("source must be a regular file within the selected repository")
        raw = p.read_bytes()
        packet[name] = {"sha256": hashlib.sha256(raw).hexdigest(),
                        "base64": base64.b64encode(raw).decode("ascii")}
    return validate_packet(packet, packet)


def _absolute(value: str | pathlib.PurePath) -> pathlib.PurePosixPath:
    text = str(value)
    p = pathlib.PurePosixPath(text)
    if not p.is_absolute() or str(p) != text or ".." in p.parts or any(ord(c) < 32 for c in text):
        raise ValueError("expected canonical absolute remote path")
    return p


def _without_options(args: list[str], names: set[str]) -> list[str]:
    result, i = [], 0
    while i < len(args):
        arg = args[i]
        if arg == "-o":
            if i + 1 >= len(args):
                raise ValueError("incomplete SSH option")
            if args[i + 1].split("=", 1)[0].lower() in names:
                i += 2
                continue
        elif arg.startswith("-o") and arg[2:].split("=", 1)[0].lower() in names:
            i += 1
            continue
        result.append(arg)
        i += 1
    return result


class RemoteHost:
    """One qualified SSH ControlMaster; no fallback connection or automatic retry."""

    def __init__(self, ssh_args_path, remote_repo, output_dir):
        self.remote_repo = _absolute(remote_repo)
        self.owned_root = self.remote_repo.parent
        self.output = pathlib.Path(output_dir).resolve()
        self.output.mkdir(mode=0o700, parents=True, exist_ok=True)
        self.args = json.loads(pathlib.Path(ssh_args_path).read_text())
        if (not isinstance(self.args, list) or len(self.args) < 2
                or any(not isinstance(x, str) or not x or any(ord(c) < 32 for c in x) for x in self.args)
                or pathlib.Path(self.args[0]).name != "ssh" or self.args[-1].startswith("-")):
            raise ValueError("invalid supplied SSH argument list")
        if any(x in {"-M", "-S", "-L", "-R", "-D", "-W"} for x in self.args[:-1]):
            raise ValueError("supplied base route must not own another master or forwarding scope")
        options = []
        for i, arg in enumerate(self.args[:-1]):
            if arg == "-o" and i + 1 < len(self.args) - 1:
                options.append(self.args[i + 1])
            elif arg.startswith("-o"):
                options.append(arg[2:])
        pins = [x.split("=", 1)[1].lower() for x in options
                if x.lower().startswith("stricthostkeychecking=")]
        if not pins or any(x != "yes" for x in pins):
            raise ValueError("supplied SSH route must explicitly retain strict host-key checking")
        if any(x.lower().startswith("userknownhostsfile=/dev/null") for x in options):
            raise ValueError("supplied SSH route discards host-key pins")
        self.master = None
        self.socket = None
        self._temp = None
        self._master_log = None
        self._cases = set()
        self.source_hashes = None

    def _private_write(self, path, raw):
        with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as f:
            f.write(raw)

    def start(self, timeout=30):
        if self.master is not None:
            raise RuntimeError("remote transport already started")
        self._temp = tempfile.TemporaryDirectory(prefix="t49-", dir="/tmp")
        self.socket = pathlib.Path(self._temp.name) / "ctl"
        self._master_log = open(self.output / "ssh-master.log", "xb")
        os.chmod(self.output / "ssh-master.log", 0o600)
        forwards = []
        for kind, local, remote in [("-L", 43149, 43149), ("-R", 43148, 43148),
                                    ("-L", 43151, 43151), ("-R", 43150, 43150)]:
            forwards += [kind, f"127.0.0.1:{local}:127.0.0.1:{remote}"]
        command = self.args[:-1] + ["-M", "-S", str(self.socket), "-N",
                    "-oExitOnForwardFailure=yes", "-oControlPersist=no"] + forwards + self.args[-1:]
        self.master = subprocess.Popen(command, stdout=self._master_log, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + timeout
        try:
            while not self.socket.exists():
                if self.master.poll() is not None:
                    raise RuntimeError("strict SSH master failed; inspect the owned private log")
                if time.monotonic() >= deadline:
                    raise TimeoutError("owned SSH master startup timed out")
                time.sleep(0.05)
            self.verify_python()
        except BaseException:
            self.close(require_terminal=False)
            raise
        return self

    def ssh(self):
        if self.master is None or self.master.poll() is not None or not self.socket.exists():
            raise RuntimeError("owned SSH master is not live")
        # Mux clients retain all host/account pins. Disable only fallback routes:
        # a missing socket must fail rather than open a second provider session.
        base = _without_options(self.args[:-1], {"proxycommand", "proxyjump"})
        return base + ["-S", str(self.socket), "-oControlMaster=no", "-oProxyCommand=false"] + self.args[-1:]

    def remote_python(self, script: str, timeout=90) -> str:
        command = shlex.join([str(self.remote_repo / ".venv/bin/python"), "-"])
        ident = uuid.uuid4().hex
        try:
            r = subprocess.run(self.ssh() + [command], input=script.encode(), capture_output=True,
                               timeout=timeout, check=False)
        except subprocess.TimeoutExpired as exc:
            self._private_write(self.output / (ident + ".stdout"), exc.stdout or b"")
            self._private_write(self.output / (ident + ".stderr"), exc.stderr or b"")
            raise TimeoutError("remote command timed out; remote outcome needs explicit readback") from None
        self._private_write(self.output / (ident + ".stdout"), r.stdout)
        self._private_write(self.output / (ident + ".stderr"), r.stderr)
        if r.returncode:
            raise RuntimeError(f"remote command exited {r.returncode}; inspect owned private log {ident}")
        return r.stdout.decode("utf-8")

    def verify_python(self):
        script = f"""import sys,pathlib,json
expected=pathlib.Path({str(self.remote_repo / '.venv')!r}).resolve()
assert sys.version_info[:3]==(3,15,2)
assert pathlib.Path(sys.prefix).resolve()==expected
print(json.dumps({{'version':sys.version,'executable':sys.executable,'prefix':sys.prefix}}))
"""
        return json.loads(self.remote_python(script, timeout=20))

    def upload_source(self, packet, allowed_paths):
        packet = validate_packet(packet, allowed_paths)
        data = json.dumps(packet, separators=(",", ":"))
        script = f"""import pathlib,json,base64,hashlib,os
repo=pathlib.Path({str(self.remote_repo)!r});packet=json.loads({data!r})
assert repo.is_dir() and not repo.is_symlink()
root=repo.resolve();written={{}}
for name,item in packet.items():
 p=repo/name
 for ancestor in [p,*p.parents]:
  if ancestor==repo.parent:break
  assert not ancestor.is_symlink(),'source symlink rejected'
 assert p.resolve().is_relative_to(root)
 raw=base64.b64decode(item['base64'],validate=True)
 assert hashlib.sha256(raw).hexdigest()==item['sha256']
 p.parent.mkdir(parents=True,exist_ok=True)
 if not p.exists() or p.read_bytes()!=raw:p.write_bytes(raw)
 written[name]=hashlib.sha256(p.read_bytes()).hexdigest()
print(json.dumps(written,sort_keys=True))
"""
        result = json.loads(self.remote_python(script, timeout=90))
        assert result == {n: v["sha256"] for n, v in packet.items()}
        self.source_hashes = result
        return result

    def cargo(self, args: Sequence[str], executable_names: Sequence[str], *, log_name=None,
              source_hashes=None, timeout=900):
        if (not args or args[0] not in {"build", "test", "check", "clippy"}
                or any(not isinstance(x, str) or any(ord(c) < 32 for c in x) for x in args)):
            raise ValueError("expected explicit Cargo build/test/check/clippy argument list")
        command = list(args)
        split = command.index("--") if "--" in command else len(command)
        command, runner_args = command[:split], command[split:]
        if "--locked" not in command:
            command.append("--locked")
        if any(x.startswith("--message-format") and x != "--message-format=json" for x in command):
            raise ValueError("Cargo artifact readback requires JSON message format")
        if "--message-format=json" not in command:
            command.append("--message-format=json")
        command += runner_args
        name = log_name or ("cargo-" + uuid.uuid4().hex + ".log")
        if not re.fullmatch(r"[A-Za-z0-9_.-]{1,80}", name) or name in {".", ".."}:
            raise ValueError("invalid owned log name")
        hashes = source_hashes if source_hashes is not None else self.source_hashes
        if not hashes:
            raise ValueError("frozen source hashes are required before a remote build")
        for n, sha in hashes.items():
            source_path(n)
            if not re.fullmatch("[0-9a-f]{64}", sha):
                raise ValueError("invalid frozen source hash")
        payload = json.dumps({"args": command, "names": list(executable_names), "hashes": hashes,
                              "log": name, "timeout": timeout}, separators=(",", ":"))
        script = f"""import pathlib,json,hashlib,subprocess,os,signal
repo=pathlib.Path({str(self.remote_repo)!r});base=repo.parent;c=json.loads({payload!r})
for n,h in c['hashes'].items():assert hashlib.sha256((repo/n).read_bytes()).hexdigest()==h
logs=base/'track49-remote-logs';logs.mkdir(mode=0o700,exist_ok=True);log=logs/c['log']
env=dict(os.environ,RUSTUP_HOME=str(base/'.rustup'),CARGO_HOME=str(base/'.cargo'),PATH=str(base/'.cargo/bin')+':'+os.environ['PATH'],CARGO_BUILD_JOBS='2',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_TARGET_DIR=str(repo/'target'),RUSTC=str(base/'.cargo/bin/rustc'),RUSTDOC=str(base/'.cargo/bin/rustdoc'),RUSTUP_TOOLCHAIN='1.99.1')
rust=subprocess.check_output([env['RUSTC'],'-vV'],env=env,text=True);assert 'release: 1.99.1\\n' in rust
cargo=subprocess.check_output([str(base/'.cargo/bin/cargo'),'--version'],env=env,text=True);assert cargo.startswith('cargo 1.99.1 ')
with os.fdopen(os.open(log,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'wb') as out:
 proc=subprocess.Popen([str(base/'.cargo/bin/cargo')]+c['args'],cwd=repo,env=env,stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
 timed_out=False
 try:rc=proc.wait(timeout=c['timeout'])
 except subprocess.TimeoutExpired:
  timed_out=True;os.killpg(proc.pid,signal.SIGKILL);rc=proc.wait(timeout=20)
assert all(hashlib.sha256((repo/n).read_bytes()).hexdigest()==h for n,h in c['hashes'].items())
assert log.stat().st_size<={MAX_LOG_BYTES},'raw Cargo log exceeds readback bound'
raw=log.read_bytes();executables={{}}
for line in raw.decode(errors='replace').splitlines():
 try:r=json.loads(line)
 except json.JSONDecodeError:continue
 if r.get('reason')=='compiler-artifact' and r.get('executable') and r.get('target',{{}}).get('name') in c['names']:
  p=pathlib.Path(r['executable']);assert p.is_file() and p.resolve().is_relative_to((repo/'target').resolve())
  value={{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'target_kind':r['target']['kind'],'package_id':r['package_id']}}
  key=r['target']['name'];assert key not in executables or executables[key]==value,'ambiguous executable target';executables[key]=value
if rc==0:assert set(executables)==set(c['names']),'missing requested compiler-artifact executable'
print(json.dumps({{'exit':rc,'timed_out':timed_out,'executables':executables,'source':c['hashes'],'rust':rust,'cargo':cargo,'remote_log':str(log),'log_sha256':hashlib.sha256(raw).hexdigest(),'log_tail':raw[-4000:].decode(errors='replace')}}))
"""
        result = json.loads(self.remote_python(script, timeout=timeout + 60))
        raw = self.fetch_log(result["remote_log"])
        assert hashlib.sha256(raw).hexdigest() == result["log_sha256"]
        self._private_write(self.output / name, raw)
        return result

    def fetch_log(self, path):
        p = _absolute(path)
        if not p.is_relative_to(self.owned_root / "track49-remote-logs"):
            raise ValueError("log outside owned Cargo log directory")
        script = f"""import pathlib,base64
p=pathlib.Path({str(p)!r});assert p.is_file() and not p.is_symlink();assert p.stat().st_size<={MAX_LOG_BYTES}
print(base64.b64encode(p.read_bytes()).decode())
"""
        return base64.b64decode(self.remote_python(script).strip(), validate=True)

    def _case(self, case_path):
        p = _absolute(case_path)
        if p == self.owned_root or not p.is_relative_to(self.owned_root) or p.is_relative_to(self.remote_repo):
            raise ValueError("native case must be inside owned provider root, outside source repository")
        return p

    def popen_owned(self, command: Sequence[str], case_path, pidfile, *, new_session=True,
                    **popen_options):
        case = self._case(case_path)
        pid = _absolute(pidfile) if str(pidfile).startswith("/") else case / str(pidfile)
        if not pid.is_relative_to(case) or pid == case or ".." in pid.parts:
            raise ValueError("PID file outside owned case")
        if not command or any(not isinstance(x, str) or not x or any(ord(c) < 32 for c in x) for x in command):
            raise ValueError("invalid owned command list")
        payload = json.dumps({"args": list(command), "case": str(case), "pidfile": str(pid),
                              "setsid": bool(new_session)}, separators=(",", ":"))
        wrapper = f"""import pathlib,json,os
c=json.loads({payload!r});case=pathlib.Path(c['case']);root=pathlib.Path({str(self.owned_root)!r}).resolve()
case.mkdir(mode=0o700,parents=True,exist_ok=True)
assert case.resolve().is_relative_to(root) and not case.is_symlink()
p=pathlib.Path(c['pidfile']);assert p.parent.resolve().is_relative_to(case.resolve()) and not p.is_symlink()
if c['setsid']:
 if os.getpgrp()==os.getpid() and os.getsid(0)!=os.getpid():
  child=os.fork()
  if child:
   _,status=os.waitpid(child,0);rc=os.waitstatus_to_exitcode(status);raise SystemExit(rc if rc>=0 else 128-rc)
 if os.getsid(0)!=os.getpid():os.setsid()
 assert os.getsid(0)==os.getpid() and os.getpgrp()==os.getpid()
with os.fdopen(os.open(p,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'w') as f:f.write(str(os.getpid())+'\\n');f.flush();os.fsync(f.fileno())
os.chdir(case);os.execvp(c['args'][0],c['args'])
"""
        remote_command = shlex.join([str(self.remote_repo / ".venv/bin/python"), "-c", wrapper])
        options = dict(stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                       text=True, bufsize=1)
        options.update(popen_options)
        proc = subprocess.Popen(self.ssh() + [remote_command], **options)
        self._cases.add(str(case))
        return proc

    def read_pidfile(self, case_path, pidfile):
        case = self._case(case_path)
        p = _absolute(pidfile) if str(pidfile).startswith("/") else case / str(pidfile)
        if not p.is_relative_to(case) or ".." in p.parts:
            raise ValueError("PID file outside owned case")
        script = f"""import pathlib,json
p=pathlib.Path({str(p)!r});assert p.is_file() and not p.is_symlink();v=p.read_text().strip();assert v.isdecimal() and int(v)>1
print(json.dumps({{'pid':int(v)}}))
"""
        return json.loads(self.remote_python(script))["pid"]

    def owned_status(self, pid: int, executable_marker: str, settingspath: str):
        return self._pid_operation(pid, executable_marker, settingspath, None, False)

    def signal_owned(self, pid: int, executable_marker: str, settingspath: str,
                     sig=signal.SIGKILL, *, group=False):
        if sig not in {signal.SIGKILL, signal.SIGTERM, signal.SIGINT}:
            raise ValueError("unsupported owned-process signal")
        return self._pid_operation(pid, executable_marker, settingspath, int(sig), group)

    def _pid_operation(self, pid, marker, settings, sig, group):
        if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 1 or not marker or any(ord(c) < 32 for c in marker):
            raise ValueError("invalid exact owned PID/marker")
        settings = _absolute(settings)
        if not settings.is_relative_to(self.owned_root):
            raise ValueError("settings path outside owned provider root")
        c = json.dumps({"pid": pid, "marker": marker, "settings": str(settings), "signal": sig,
                        "group": bool(group)}, separators=(",", ":"))
        script = f"""import pathlib,json,os,signal
c=json.loads({c!r});p=pathlib.Path('/proc')/str(c['pid'])
try:raw=(p/'cmdline').read_bytes();stat=(p/'stat').read_text()
except FileNotFoundError:print(json.dumps({{'pid':c['pid'],'exists':False}}));raise SystemExit
argv=[x.decode(errors='replace') for x in raw.split(b'\\0') if x]
state=stat[stat.rfind(')')+2:].split()[0]
matched=any(c['marker'] in x for x in argv) and c['settings'] in argv
result={{'pid':c['pid'],'exists':True,'state':state,'matched':matched,'process_group':os.getpgid(c['pid'])}}
if c['signal'] is not None:
 assert matched and state!='Z','PID marker/settings ownership guard rejected'
 if c['group']:
  assert os.getpgid(c['pid'])==c['pid'],'refusing a foreign process group';os.killpg(c['pid'],c['signal'])
 else:os.kill(c['pid'],c['signal'])
 result['signal_sent']=c['signal']
print(json.dumps(result))
"""
        return json.loads(self.remote_python(script))

    def terminal_statuses(self):
        script = f"""import pathlib,json
cases=json.loads({json.dumps(sorted(self._cases))!r});active=[]
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdecimal():continue
 try:
  argv=[x.decode(errors='replace') for x in (p/'cmdline').read_bytes().split(b'\\0') if x];stat=(p/'stat').read_text();state=stat[stat.rfind(')')+2:].split()[0]
 except (FileNotFoundError,PermissionError,ProcessLookupError):continue
 if state!='Z' and any(a==c or a.startswith(c+'/') for a in argv for c in cases):active.append(int(p.name))
print(json.dumps({{'active_owned_case_pids':active}}))
"""
        return json.loads(self.remote_python(script))

    def close(self, *, require_terminal=True):
        if self.master is not None and self.master.poll() is None:
            if require_terminal and self._cases and self.terminal_statuses()["active_owned_case_pids"]:
                raise RuntimeError("owned native processes remain; explicit guarded termination is required")
            self.master.terminate()
            try:
                self.master.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.master.kill()
                self.master.wait(timeout=10)
        if self._master_log is not None:
            self._master_log.close()
        if self._temp is not None:
            self._temp.cleanup()
        self.socket = None

    def __enter__(self):
        return self.start()

    def __exit__(self, *_):
        self.close()
