#!/usr/bin/env python3
"""Owned synthetic CAP48 native qualification on one or two hosts.
Run with the repository's verified .venv/bin/python. Never uses provider tokens.
Private keys stay on their generating host; only public certificates and source
are transferred. CLI session allocation/termination remains the calling owner.
"""
from __future__ import annotations
import argparse, base64, hashlib, json, os, pathlib, queue, shlex, subprocess, sys, shutil, tempfile, threading, time, uuid
if sys.version_info[:3] != (3,15,2):
    raise SystemExit("Select the qualified repository-local Python3.15.2")
ROOT=pathlib.Path(__file__).resolve().parents[2]
FILES=['Cargo.toml','rust-toolchain.toml','Cargo.lock','crates/kairo-ecs-pdes/Cargo.toml','crates/kairo-ecs-pdes/src/lib.rs','crates/kairo-ecs-pdes/src/optimistic.rs','crates/kairo-ecs-pdes/src/optimistic/external.rs','crates/kairo-ecs-pdes/src/optimistic/external/protocol.rs','crates/kairo-ecs-pdes/src/optimistic/external/tests.rs','crates/kairo-ecs-pdes/src/optimistic/external_store.rs','crates/kairo-ecs-pdes/src/optimistic/external_store/tests.rs','crates/kairo-ecs-grpc/Cargo.toml','crates/kairo-ecs-grpc/src/lib.rs','crates/kairo-ecs-grpc/src/external_transport.rs','crates/kairo-ecs-grpc/tests/external_tls.rs','crates/kairo-ecs-grpc/examples/cap48_node.rs']
BINARY=pathlib.Path(os.environ.get('CARGO_TARGET_DIR','/Volumes/PortableSSD/codex-builds/kairos-track48-339aba1'))/'debug/examples/cap48_node'
GROUP_CASE = "three-member-cut"
def checked(args,**kw):
    return subprocess.run(args,check=True,text=True,capture_output=True,timeout=kw.pop('timeout',60),**kw)
def private_dir(path):
    path.mkdir(mode=0o700,parents=True,exist_ok=False)
def pki_script(path,name):
    # This program is executed on the key's own host, never exports key material.
    return f'''
import pathlib,subprocess,json,base64,hashlib
p=pathlib.Path({str(path)!r});p.mkdir(mode=0o700,parents=True,exist_ok=True)
(p/'ca.conf').write_text('[req]\\ndistinguished_name=dn\\nx509_extensions=v3_ca\\n[dn]\\nCN=fixture\\n[v3_ca]\\nbasicConstraints=critical,CA:TRUE\\nkeyUsage=critical,keyCertSign,cRLSign\\nsubjectKeyIdentifier=hash\\n')
(p/'leaf.ext').write_text('basicConstraints=critical,CA:FALSE\\nkeyUsage=critical,digitalSignature\\nextendedKeyUsage=serverAuth,clientAuth\\nsubjectAltName=DNS:{name}\\n')
commands=[['req','-x509','-newkey','rsa:2048','-nodes','-subj','/CN={name}-CA','-keyout','ca.key','-out','ca.pem','-days','1','-config','ca.conf'],['req','-new','-newkey','rsa:2048','-nodes','-subj','/CN={name}','-keyout','leaf.key','-out','leaf.csr'],['pkcs8','-topk8','-nocrypt','-in','leaf.key','-out','leaf.pk8'],['x509','-req','-in','leaf.csr','-CA','ca.pem','-CAkey','ca.key','-CAcreateserial','-out','leaf.pem','-days','1','-extfile','leaf.ext'],['x509','-in','leaf.pem','-outform','DER','-out','leaf.der']]
for cmd in commands:
 r=subprocess.run(['openssl']+cmd,cwd=p,capture_output=True,text=True,timeout=30);assert r.returncode==0,r.stderr
print(json.dumps({{'ca':(p/'ca.pem').read_text(),'certificate':(p/'leaf.pem').read_text(),'hash':list(hashlib.sha256((p/'leaf.der').read_bytes()).digest())}}))
'''
class Node:
    def __init__(self,owner,participant,settings,remote=False):
        self.owner=owner;self.participant=participant;self.settings=settings;self.remote=remote;self.responses=queue.Queue();self.boundaries=queue.Queue();self.command_lock=threading.Lock()
        self.pid = None
        if remote:
            command = ("""printf '{"worker_pid":%d}\n' "$$"; """
                       f'exec {shlex.quote(str(owner.remote_repo/"target/debug/examples/cap48_node"))} '
                       f'{shlex.quote(str(settings))}')
            args=owner.ssh()+[command]
        else:args=[str(BINARY),str(settings)]
        self.stderr=(owner.output/f'node-{participant}-{uuid.uuid4().hex[:8]}.stderr').open('w')
        self.proc=subprocess.Popen(args,text=True,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr,bufsize=1)
        owner.nodes.append(self)
        self.reader=threading.Thread(target=self._read,daemon=True);self.reader.start();self.ready=self.responses.get(timeout=30);assert self.ready.get('ready'),self.ready
        pid = int(self.ready['pid'])
        assert self.pid is None or self.pid == pid
        self.pid = pid
        owner.event('worker_ready',participant=participant,pid=self.pid,remote=remote,generation=self.ready['generation'])
    def _read(self):
        for line in self.proc.stdout:
            if 'STORE_BOUNDARY' in line:self.boundaries.put(line.strip());continue
            try:
                response = json.loads(line)
                if self.remote and isinstance(response, dict) and response.keys() == {'worker_pid'}:
                    self.pid = int(response['worker_pid'])
                else:
                    self.responses.put(response)
            except json.JSONDecodeError:self.owner.event('native_stdout',line=line[:500],participant=self.participant)
    def call(self,op,expect=True,**fields):
        with self.command_lock:
            assert self.proc.poll() is None,'worker exited before command'
            self.proc.stdin.write(json.dumps(dict(op=op,**fields),separators=(',',':'))+'\n');self.proc.stdin.flush()
            response=self.responses.get(timeout=40)
        self.owner.event('command',participant=self.participant,operation=op,ok=response.get('ok'),error=response.get('error'))
        if expect:assert response.get('ok'),response
        return response['result'] if response.get('ok') else response
    def kill(self):
        if self.remote:
            # The SSH shell execs the worker at the same PID. Verify the exact
            # executable and settings, then signal the same Linux process.
            binary = self.owner.remote_repo / 'target/debug/examples/cap48_node'
            script = f'''import os,pathlib,signal
pid={int(self.pid)}
assert pid > 1
proc=pathlib.Path('/proc')/str(pid)
fd=os.pidfd_open(pid)
try:
    assert (proc/'exe').resolve(strict=True) == pathlib.Path({str(binary)!r}).resolve(strict=True)
    assert (proc/'cmdline').read_bytes().split(b'\\0')[1:2] == [{str(self.settings)!r}.encode()]
    signal.pidfd_send_signal(fd,signal.SIGKILL)
finally:
    os.close(fd)
'''
            self.owner.remote_python(script)
        else:self.proc.kill()
        rc=self.proc.wait(timeout=20);assert rc!=0;self.stderr.close();self.owner.event('worker_sigkill',participant=self.participant,pid=self.pid,exit=rc)
    def stop(self):
        try:
            if self.proc.poll() is not None:
                return
            try:
                self.proc.stdin.write('{"op":"stop"}\n')
                self.proc.stdin.flush()
            except BrokenPipeError:
                pass
            try:
                self.proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                if not self.remote or self.pid is not None:
                    self.kill()
                else:
                    self.proc.kill()
                    self.proc.wait(timeout=20)
                raise
        finally:
            self.stderr.close()
class Owner:
    def __init__(self,args):
        self.args=args;self.output=pathlib.Path(args.output).resolve();self.output.mkdir(mode=0o700,parents=True,exist_ok=True);self.events=(self.output/'events.jsonl').open('a');self.nodes=[];self.master=None;self.socket=None;self.session_temp=None
        self.remote_repo=pathlib.Path(args.remote_repo)
        self.remote_prefix=json.loads(pathlib.Path(args.ssh_args).read_text()) if args.ssh_args else None
        self.participants = range(3 if args.cases == GROUP_CASE else 2)
        self.ports = [43148 + participant for participant in self.participants]
        self.run_id = uuid.uuid4().hex
        self.local_base = pathlib.Path(tempfile.mkdtemp(prefix='cap48-full-'))
        self.remote_base = (self.remote_repo.parent / f'qualification-{self.run_id}'
                            if self.remote_prefix else self.local_base / 'remote')
    def event(self,event,**kw):self.events.write(json.dumps(dict(event=event,monotonic=time.monotonic(),**kw))+'\n');self.events.flush()
    def ssh(self):
        assert self.remote_prefix
        return self.remote_prefix[:-1]+(['-S',str(self.socket)] if self.socket else [])+self.remote_prefix[-1:]
    def remote_python(self,script,timeout=90):
        python = shlex.quote(str(self.remote_repo / '.venv/bin/python'))
        return checked(self.ssh()+[python+' -'],input=script,timeout=timeout).stdout
    def setup(self):
        self.frozen_files=FILES+['docs/distributed/qualify_cap48.py']+[str(p.relative_to(ROOT)) for p in (ROOT/'.cargo').glob('*') if p.is_file() and checked(['git','ls-files','--error-unmatch',str(p.relative_to(ROOT))],cwd=ROOT).returncode==0]
        self.frozen_source={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in self.frozen_files}
        build=checked(['cargo','build','--locked','--message-format=json','-p','kairo-ecs-grpc','--example','cap48_node','--features','external-fault-injection'],cwd=ROOT,env=dict(os.environ,CARGO_TARGET_DIR=str(BINARY.parents[2]),RUSTC=shutil.which('rustc'),RUSTDOC=shutil.which('rustdoc'),RUSTUP_TOOLCHAIN='1.99.1'),timeout=600)
        (self.output/'local-build.log').write_text(build.stdout+build.stderr)
        produced=[json.loads(line) for line in build.stdout.splitlines() if line[:1]==chr(123)];executables=[r['executable'] for r in produced if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='cap48_node' and r.get('executable')];assert executables and pathlib.Path(executables[-1]).resolve()==BINARY.resolve()
        (self.output/'local-rust.txt').write_text(checked(['rustc','-vV']).stdout)
        assert self.frozen_source=={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in self.frozen_files}
        self.frozen_binary=hashlib.sha256(BINARY.read_bytes()).hexdigest()
        self.event('fixture_ownership',local_base=str(self.local_base),remote_base=str(self.remote_base))
        self.event('setup',remote=bool(self.remote_prefix),source_base=checked(['git','rev-parse','HEAD'],cwd=ROOT).stdout.strip())
        if self.remote_prefix:
            self.session_temp=tempfile.TemporaryDirectory(prefix='cap48-ssh-',dir='/tmp');self.socket=pathlib.Path(self.session_temp.name)/'ctl'
            forwards = ['-R', f'127.0.0.1:{self.ports[0]}:127.0.0.1:{self.ports[0]}']
            for port in self.ports[1:]:
                forwards.extend(['-L', f'127.0.0.1:{port}:127.0.0.1:{port}'])
            args=self.remote_prefix[:-1]+['-M','-S',str(self.socket),'-N','-oExitOnForwardFailure=yes','-oControlPersist=no']+forwards+self.remote_prefix[-1:]
            log=(self.output/'ssh-master.log').open('w');self.master=subprocess.Popen(args,stdout=log,stderr=subprocess.STDOUT)
            deadline=time.monotonic()+30
            while not self.socket.exists():
                if self.master.poll() is not None:raise RuntimeError('strict SSH master failed; see log')
                if time.monotonic()>deadline:raise TimeoutError('SSH master startup; see ssh-master.log')
                time.sleep(.05)
            ssh = self.ssh()
            checked(ssh[:-1]+['-O','check',ssh[-1]],timeout=10)
            prefix = str(self.remote_repo / '.venv')
            self.remote_python(f'import sys; assert sys.version_info[:3] == (3, 15, 2)'
                               f' and sys.prefix == {prefix!r}; print("CAP48_PINNED_SSH_READY")')
            packet={name:{'sha256':hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),'bytes':base64.b64encode((ROOT/name).read_bytes()).decode()} for name in FILES}
            # Frozen public source only. No sessions, tokens, keys or native stores.
            script=f'''import pathlib,json,base64,hashlib,subprocess,os
repo=pathlib.Path({str(self.remote_repo)!r}); packet=json.loads({json.dumps(packet)!r})
for name,item in packet.items():
 raw=base64.b64decode(item['bytes']);assert hashlib.sha256(raw).hexdigest()==item['sha256'];p=repo/name;p.parent.mkdir(parents=True,exist_ok=True)
 if not p.exists() or p.read_bytes()!=raw:p.write_bytes(raw)
base=repo.parent;env=dict(os.environ,RUSTUP_HOME=str(base/'.rustup'),CARGO_HOME=str(base/'.cargo'),PATH=str(base/'.cargo/bin')+':'+os.environ['PATH'],CARGO_BUILD_JOBS='2',CARGO_TARGET_DIR=str(repo/'target'),RUSTC=str(base/'.cargo/bin/rustc'),RUSTDOC=str(base/'.cargo/bin/rustdoc'),RUSTUP_TOOLCHAIN='1.99.1')
log=base/'full-node-build.log'
with log.open('w') as stream:
 r=subprocess.run([str(base/'.cargo/bin/cargo'),'build','--locked','--message-format=json','-p','kairo-ecs-grpc','--example','cap48_node','--features','external-fault-injection'],cwd=repo,env=env,stdout=stream,stderr=subprocess.STDOUT,timeout=600)
artifacts=[json.loads(line) for line in log.read_text().splitlines() if line[:1]==chr(123)];executables=[x['executable'] for x in artifacts if x.get('reason')=='compiler-artifact' and x.get('target',{{}}).get('name')=='cap48_node' and x.get('executable')];assert executables and pathlib.Path(executables[-1])==repo/'target/debug/examples/cap48_node'
assert all(hashlib.sha256((repo/n).read_bytes()).hexdigest()==v['sha256'] for n,v in packet.items())
print(json.dumps({{'exit':r.returncode,'binary_sha256':hashlib.sha256(pathlib.Path(executables[-1]).read_bytes()).hexdigest(),'log':log.read_text()[-4000:],'source':{{n:v['sha256'] for n,v in packet.items()}},'rust':subprocess.check_output([str(base/'.cargo/bin/rustc'),'-vV'],env=env,text=True)}}));assert r.returncode==0
'''
            result=json.loads(self.remote_python(script,timeout=660));(self.output/'remote-build.json').write_text(json.dumps(result,indent=2));assert result['exit']==0
        self.description=json.loads(checked([str(BINARY),'--describe',str(len(self.participants))]).stdout)
        self.public=[]
        for participant in self.participants:
            base = self.remote_base if participant != 0 and self.remote_prefix else self.local_base
            script = pki_script(base / f'pki-{participant}', f'node{participant}')
            if base == self.remote_base:
                public = self.remote_python(script)
            else:
                public = checked([str(ROOT/'.venv/bin/python'),'-'],input=script).stdout
            self.public.append(json.loads(public))
        self.config={'simulation':[48]*16,'model':self.description['model'],'members':[{'participant':p,'public_key':self.description['public_keys'][p],'certificate_hash':self.public[p]['hash'],'lps':[p]} for p in self.participants],'topology':[[p,[1 if p == 0 else 0]] for p in self.participants]}
        self.event('source_and_pki_ready',source={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in FILES},native_binary=hashlib.sha256(BINARY.read_bytes()).hexdigest())
    def workers(self,name):
        result=[]
        config=json.loads(json.dumps(self.config));config['simulation']=list(hashlib.sha256((self.run_id+':'+name).encode()).digest()[:16]);(self.output/f'{name}-configuration.json').write_text(json.dumps(config,indent=2))
        for p in self.participants:
            remote = p != 0 and bool(self.remote_prefix)
            host_dir = self.remote_base if remote else self.local_base
            base = host_dir / name / f'node{p}'
            pki = host_dir / f'pki-{p}'
            peer = 1 if p == 0 else 0
            settings = {
                'configuration': config, 'participant': p, 'store': str(base / 'store'),
                'listen': f'127.0.0.1:{self.ports[p]}', 'certificate': str(pki / 'leaf.pem'),
                'private_key': str(pki / 'leaf.pk8'), 'client_ca': str(base / 'client-ca.pem'),
                'peer_endpoint': f'https://127.0.0.1:{self.ports[peer]}', 'peer_domain': f'node{peer}',
                'peer_ca': str(base / 'peer-ca.pem'), 'peer_certificate_hash': self.public[peer]['hash'],
            }
            roots = {str(base / 'client-ca.pem'): ''.join(entry['ca'] for entry in self.public),
                     str(base / 'peer-ca.pem'): self.public[peer]['ca']}
            if p == 0 and len(self.participants) == 3:
                extra_ca = str(base / 'peer-2-ca.pem')
                roots[extra_ca] = self.public[2]['ca']
                settings['peers'] = {2: {'endpoint': f'https://127.0.0.1:{self.ports[2]}',
                                          'domain': 'node2', 'ca': extra_ca,
                                          'certificate_hash': self.public[2]['hash']}}
            script=f'''import pathlib,json
base=pathlib.Path({str(base)!r});(base/'store').mkdir(mode=0o700,parents=True,exist_ok=False)
for path, certs in {roots!r}.items():pathlib.Path(path).write_text(certs)
(base/'settings.json').write_text({json.dumps(settings)!r})
'''
            if remote:self.remote_python(script)
            else:checked([str(ROOT/'.venv/bin/python'),'-'],input=script)
            result.append(Node(self,p,base/'settings.json',remote))
        return result
    def restart(self,node):
        return Node(self,node.participant,node.settings,node.remote)
    def close(self):
        for node in self.nodes:
            try:node.stop()
            except Exception as e:self.event('cleanup_error',error=str(e),participant=node.participant)
        if self.master:
            self.master.terminate()
            try:self.master.wait(timeout=10)
            except subprocess.TimeoutExpired:self.master.kill();self.master.wait(timeout=10)
        if self.session_temp:self.session_temp.cleanup()
        self.events.close()
def decode(packet):return json.loads(bytes(packet))['record']
def message(packet):return decode(packet)['body'].get('message')
def tick(m):return int(m['tick'],16)
def image(node):return json.loads(bytes(node.call('snapshot',lp=node.participant)['checkpoint']))
def pending(node):return node.call('pending')['packets']
def choose(node,condition):return next(p for p in pending(node) if (m:=message(p)) is not None and condition(m))
def receive_local(node,packet):
    generation=node.call('snapshot')['generation']
    ingress='recover' if decode(packet)['generation']<generation else 'receive'
    replies=node.call(ingress,packet=packet)['replies']
    for reply in replies:node.call('receive',packet=reply)
    return replies
def join(a,b):
    p=a.call('propose')['packet'];ja=a.call('join',packet=p)['packet'];jb=a.call('send',packet=p,accept=False)['replies'][0];bundle=[ja,jb];a.call('activate',packet=p,joins=bundle);b.call('activate',packet=p,joins=bundle)
def send(sender,packet,accept=True):return sender.call('send',packet=packet,accept=accept)['replies']
def root(a,seq,t,op):
    a.call('root',sequence=seq,source=0,destination=0,tick=t,payload=[op]);return choose(a,lambda m:m['root_sequence']==seq and not m['ancestry'])
def models(a,b):return [bytes(a.call('snapshot',lp=0)['model']),bytes(b.call('snapshot',lp=1)['model'])]


def recover_group_in_flight(nodes, restart, first_peer=None):
    """Run the three-member case on configured native workers and private stores.

    Members 0 and 1 use their default mTLS route to each other. Member 0 also
    has an explicit route to member 2, whose default route returns to 0.
    Restart kills and reopens the selected worker with the same settings.
    If first_peer is set, that worker applies the cut before the coordinator crashes.
    """
    if first_peer not in (None, 1, 2):
        raise ValueError("first_peer must be 1, 2 or None")
    a, b, c = nodes

    def send_control(peer, packet):
        route = {} if peer == 1 else {"peer": peer}
        return a.call("send", **route, packet=packet, accept=False)["replies"]

    def join_all():
        proposal = a.call("propose")["packet"]
        joins = [
            a.call("join", packet=proposal)["packet"],
            send_control(1, proposal)[0],
            send_control(2, proposal)[0],
        ]
        incomplete = a.call("activate", expect=False, packet=proposal, joins=joins[:2])
        assert incomplete["ok"] is False, incomplete
        for node in (a, b, c):
            node.call("activate", packet=proposal, joins=joins)

    def finish_cut(packet):
        send_control(1, packet)
        send_control(2, packet)
        return [int(node.call("snapshot")["gvt"], 16) for node in (a, b, c)]

    join_all()
    receive_local(a, root(a, 0, 20, 1))
    a.call("run", horizon=100, budget=1)
    send(a, choose(a, lambda event: tick(event) == 30 and not event["anti"]))
    b.call("run", horizon=100, budget=1)
    send(b, choose(b, lambda event: tick(event) == 35 and not event["anti"]))
    a.call("run", horizon=100, budget=1)
    c.call("root", sequence=0, source=2, destination=0, tick=10, payload=[2])
    [straggler] = pending(c)
    for node in (a, b, c):
        node.call("close")

    pause = a.call("pause_capture")["packet"]
    send_control(1, pause)
    prepares = [node.call("prepare", limit=100)["packet"] for node in (a, b)]
    before = a.call("snapshot")
    partial = a.call("cut", expect=False, prepares=prepares)
    assert partial["ok"] is False and "incomplete prepares" in partial["error"], partial
    after = a.call("snapshot")
    assert before["revision"] == after["revision"] and int(after["gvt"], 16) == 0

    a, c = restart(a), restart(c)
    assert a.call("prepare", limit=100)["packet"] == prepares[0]
    assert a.call("retained_pause")["packet"] == pause
    send_control(2, pause)
    prepares.append(c.call("prepare", limit=100)["packet"])
    decision = a.call("cut", prepares=prepares)["packet"]
    expected_floors = [10, 0, 0]
    waiting_peer = 2
    if first_peer is not None:
        send_control(first_peer, decision)
        expected_floors[first_peer] = 10
        waiting_peer = 3 - first_peer
        acknowledged = (a, b, c)[first_peer]
        acknowledged_revision = acknowledged.call("snapshot")["revision"]
    mixed_floors = [int(node.call("snapshot")["gvt"], 16) for node in (a, b, c)]
    assert mixed_floors == expected_floors, mixed_floors
    if first_peer is not None:
        coordinator_revision = a.call("snapshot")["revision"]
        a = restart(a)
        retained = a.call("cut_decision")["packet"]
        assert retained == decision
        recovered = a.call("snapshot")
        assert recovered["revision"] == coordinator_revision and not recovered["active"]
        assert int(recovered["gvt"], 16) == 10
        decision = retained

    waiting = (b, c)[waiting_peer - 1]
    prepared_revision = waiting.call("snapshot")["revision"]
    waiting = restart(waiting)
    if waiting_peer == 1:
        b = waiting
    else:
        c = waiting
    assert waiting.call("prepare", limit=100)["packet"] == prepares[waiting_peer]
    assert waiting.call("snapshot")["revision"] == prepared_revision
    floors = finish_cut(decision)
    assert floors == [10, 10, 10], floors
    if first_peer is not None:
        assert acknowledged.call("snapshot")["revision"] == acknowledged_revision
    standby = waiting.call("snapshot")
    assert standby["active"] is False
    send_control(waiting_peer, decision)
    assert waiting.call("snapshot")["revision"] == standby["revision"]
    assert waiting.call("run", expect=False, horizon=100, budget=1)["ok"] is False
    join_all()

    receipts = send(c, straggler)
    admitted = a.call("snapshot")["revision"]
    assert receipts and send(c, straggler) == receipts
    assert a.call("snapshot")["revision"] == admitted
    a.call("run", horizon=100, budget=3)
    anti = choose(a, lambda event: tick(event) == 30 and event["anti"])
    send(a, anti)
    b.call("run", horizon=100, budget=1)
    descendant = choose(b, lambda event: tick(event) == 35 and event["anti"])
    send(b, descendant)
    a.call("run", horizon=100, budget=4)
    send(b, descendant)
    send(a, anti)
    replacement = choose(a, lambda event: tick(event) == 25 and not event["anti"])
    send(a, replacement)
    b.call("run", horizon=100, budget=4)
    send(b, choose(b, lambda event: tick(event) == 30 and not event["anti"]))
    a.call("run", horizon=100, budget=4)
    final_models = models(a, b) + [bytes(c.call("snapshot", lp=2)["model"])]
    assert final_models[:2] == [bytes(model) for model in a.call("serial")["models"]]
    assert json.loads(final_models[2]) == {"value": 0, "rng": 48, "generation": 1, "trace": []}
    assert all(not pending(node) for node in (a, b, c))
    rollbacks = [image(node)["rollbacks"] for node in (a, b, c)]
    assert rollbacks == [1, 1, 0], rollbacks

    pause = a.call("pause_capture")["packet"]
    send_control(1, pause)
    send_control(2, pause)
    prepares = [node.call("prepare", limit=100)["packet"] for node in (a, b, c)]
    final_floors = finish_cut(a.call("cut", prepares=prepares)["packet"])
    assert final_floors == [100, 100, 100], final_floors
    return {
        "first_peer": first_peer,
        "floors_during_partial_commit": mixed_floors,
        "cut_decision_sha256": hashlib.sha256(bytes(decision)).hexdigest(),
        "floors_after_restart": floors,
        "final_floors": final_floors,
        "model_sha256": [hashlib.sha256(model).hexdigest() for model in final_models],
        "rollback_counts": rollbacks,
    }


def fault_local(owner,node,op,after,**kw):
    node.call('fault',after=after);result=[]
    def run():
        try:result.append(node.call(op,expect=False,**kw))
        except Exception as e:result.append({'terminated':type(e).__name__})
    thread=threading.Thread(target=run,daemon=True);thread.start();node.boundaries.get(timeout=20);node.kill();thread.join(timeout=1);owner.event('fault_boundary',participant=node.participant,operation=op,after=after);return owner.restart(node)
def fault_receive(owner,sender,receiver,packet,after):
    receiver.call('fault',after=after);out=[]
    def request():out.append(sender.call('send',packet=packet,expect=False))
    thread=threading.Thread(target=request,daemon=True);thread.start();receiver.boundaries.get(timeout=20);receiver.kill();thread.join(timeout=20);assert not thread.is_alive();assert out and not out[0].get('ok',True),out
    owner.event('uncertain_rpc_observed',participant=receiver.participant,after=after,result=out[0]);return owner.restart(receiver)
def semantic(node):
    state=image(node)
    name=node.settings.parent.parent.name; (node.owner.output/f'{name}-node{node.participant}-checkpoint.json').write_text(json.dumps(state,indent=2))
    return {'processes':state['processes'],'next_incarnation':state['next_incarnation'],'roots':state['roots'],'known':state['known'],'executions':state['executions'],'rollbacks':state['rollbacks'],'replays':state['replays'],'outgoing_messages':[o['message'] for o in state['outgoing']]}
def scenario(owner,name,stage=None,after=None):
    a,b=owner.workers(name)
    try:
        if stage=='activation':
            p=a.call('propose')['packet'];ja=a.call('join',packet=p)['packet'];jb=send(a,p,False)[0];bundle=[ja,jb]
            a=fault_local(owner,a,'activate',after,packet=p,joins=bundle);a.call('activate',packet=p,joins=bundle);b.call('activate',packet=p,joins=bundle);join(a,b)
        else:join(a,b)
        p20=root(a,0,20,1);p10=root(a,1,10,2);p40=root(a,2,40,9)
        receive_local(a,p20);r=a.call('snapshot')['revision'];receive_local(a,p20);assert a.call('snapshot')['revision']==r
        a.call('run',horizon=100,budget=1);old=choose(a,lambda m:tick(m)==30 and not m['anti'])
        if stage=='admission':
            b=fault_receive(owner,a,b,old,after);state=image(b);exists=any(f['packet']==old for f in state['facts']);assert exists==after,(stage,after,exists);original_response=next((f['responses'] for f in state['facts'] if f['packet']==old),None)
            join(a,b);rev=b.call('snapshot')['revision'];readback=send(a,old);assert (b.call('snapshot')['revision']==rev)==after
            if after:assert readback==original_response, 'lost ACK changed exact committed fact bytes'
        else:send(a,old)
        # Delay and duplicate exact delivery: no effect, receipt or revision repeat.
        rev=b.call('snapshot')['revision'];send(a,old);assert b.call('snapshot')['revision']==rev
        if stage=='effect':
            b=fault_local(owner,b,'run',after,horizon=100,budget=1);assert image(b)['executions']==int(after);join(a,b)
        b.call('run',horizon=100,budget=1);assert image(b)['executions']==1
        child=choose(b,lambda m:tick(m)==35 and not m['anti']);send(b,child);a.call('run',horizon=100,budget=1)
        receive_local(a,p10);a.call('run',horizon=100,budget=3)
        state=image(a);assert any(tick(o['message'])==25 and o['dependencies'] for o in state['outgoing']);assert not any(tick(message(p))==25 for p in pending(a))
        anti=choose(a,lambda m:m['anti'] and tick(m)==30);send(a,anti)
        if stage=='retirement':
            b=fault_local(owner,b,'run',after,horizon=100,budget=1);assert image(b)['rollbacks']==int(after);join(a,b)
        b.call('run',horizon=100,budget=1)
        assert any(r['applied'] and r['receipt'] is None for r in image(b)['reversals'])
        assert not any(tick(message(p))==25 for p in pending(a))
        descendant=choose(b,lambda m:m['anti'] and tick(m)==35);send(b,descendant);a.call('run',horizon=100,budget=4)
        if stage=='retirement-proof':
            b=fault_local(owner,b,'send',after,packet=descendant);assert any(r['receipt'] is not None for r in image(b)['reversals'])==after;join(a,b)
        send(b,descendant)
        # The old receiver now has complete descendant closure, but replacement
        # must remain blocked until the sender durably applies that proof.
        assert any(r['receipt'] is not None for r in image(b)['reversals'])
        if stage=='release':
            a=fault_local(owner,a,'send',after,packet=anti);assert any(o['message']['anti'] and o['retired'] for o in image(a)['outgoing'])==after;join(a,b)
        send(a,anti)
        replacement=choose(a,lambda m:tick(m)==25 and not m['anti']);assert message(replacement)['incarnation']>message(old)['incarnation']
        send(a,replacement);b.call('run',horizon=100,budget=4)
        newchild=choose(b,lambda m:tick(m)==30 and not m['anti']);send(b,newchild);a.call('run',horizon=100,budget=4)
        actual=models(a,b);serial=a.call('serial');baseline=[bytes(x) for x in serial['models']];assert actual==baseline,[(json.loads(x),json.loads(y)) for x,y in zip(actual,baseline)]
        # Compare the complete logical/physical incarnation reservation and
        # replay state with an uninterrupted distributed run, independently of
        # session/control-wire signatures which correctly change on recovery.
        all_outputs=[o['message'] for node in [a,b] for o in image(node)['outgoing'] if not o['message']['anti']]
        for expected_message in serial['messages']:
            actual_message=next(m for m in all_outputs if all(m[k]==expected_message[k] for k in ['source','destination','tick','payload']))
            assert all(actual_message[k]==expected_message[k] for k in ['root_source','root_sequence','ancestry']), 'native complete lineage mismatch'
        semantics=[semantic(a),semantic(b)]
        receive_local(a,p40);a.call('close');b.call('close');pause=a.call('pause')['packet'];send(a,pause)
        pa=a.call('prepare',limit=100)['packet'];pb=b.call('prepare',limit=100)['packet'];assert not a.call('cut',prepares=[pa],expect=False).get('ok',True)
        assert a.call('pending',expect=False).get('ok') is False
        if stage=='cut':
            a=fault_local(owner,a,'cut',after,prepares=[pa,pb]);saved=a.call('cut_decision')['packet'];assert bool(saved)==after
            decision=saved if saved else a.call('cut',prepares=[pa,pb])['packet']
        else:decision=a.call('cut',prepares=[pa,pb])['packet']
        send(a,decision);assert int(a.call('snapshot')['gvt'],16)==40 and int(b.call('snapshot')['gvt'],16)==40
        if stage=='cut':join(a,b)
        # Strict equality remains executable; a signed pre-floor and a stale
        # unretained live record must both reject, without revision mutation.
        a.call('run',horizon=100,budget=1);model=json.loads(bytes(a.call('snapshot',lp=0)['model']));expected=json.loads(actual[0]);expected['trace'].append([40,[9]]);expected['generation']+=1;expected['rng']=(expected['rng']*6364136223846793005+1)%(1<<64);assert model==expected;assert bytes(b.call('snapshot',lp=1)['model'])==actual[1]
        invalid=decode(p40);invalid['generation']=a.call('snapshot')['generation'];invalid['sequence']+=500;invalid['body']['message']['tick']=f'{39:032x}';invalid['body']['message']['root_sequence']=500;invalid['body']['message']['incarnation']=500
        signed=a.call('sign_fixture',record=invalid)['packet'];rev=a.call('snapshot')['revision'];assert not a.call('receive',packet=signed,expect=False).get('ok',True);assert a.call('snapshot')['revision']==rev
        generation=a.call('snapshot')['generation']
        if generation>1:
            stale=decode(p40);stale['generation']=generation-1;stale['sequence']+=501;stale['body']['message'].update(tick=f'{40:032x}',root_sequence=501,incarnation=501,destination=1)
            packet=a.call('sign_fixture',record=stale)['packet'];before=b.call('snapshot')['revision'];rejection=a.call('send',packet=packet,expect=False);assert not rejection.get('ok',True);assert b.call('snapshot')['revision']==before;reason=b.call('receive',packet=packet,expect=False);assert not reason.get('ok',True) and 'stale live session' in reason['error'];assert b.call('snapshot')['revision']==before
        result={'name':name,'stage':stage,'after':after,'models_sha256':[hashlib.sha256(x).hexdigest() for x in actual],'semantics':semantics,'generations':[a.call('snapshot')['generation'],b.call('snapshot')['generation']],'floor':40,'native_single_host_parity':True}
        (owner.output/f'{name}.json').write_text(json.dumps(result,indent=2));owner.event('scenario_pass',name=name,stage=stage,after=after,generations=result['generations']);return result
    finally:
        a.stop();b.stop()
def anti_before_positive(owner):
    a,b=owner.workers('anti-before-positive')
    try:
        join(a,b);p20=root(a,0,20,1);p10=root(a,1,10,2);p40=root(a,2,40,9)
        receive_local(a,p20);a.call('run',horizon=100,budget=1);old=choose(a,lambda m:tick(m)==30 and not m['anti'])
        receive_local(a,p10);a.call('run',horizon=100,budget=3);anti=choose(a,lambda m:m['anti'] and tick(m)==30)
        assert not any(tick(message(p))==25 for p in pending(a));send(a,anti);b.call('run',horizon=100,budget=1);send(a,anti)
        assert image(b)['executions']==0 and image(b)['processes'][0]['tombstones']
        # A premature pause cannot drop the suppressed, unadmitted old positive.
        a.call('close');b.call('close');pause=a.call('pause')['packet'];send(a,pause);assert not a.call('prepare',limit=100,expect=False).get('ok',True);assert int(a.call('snapshot')['gvt'],16)==0
        # Complete-quorum recovery abandons the uncommitted pause at the old
        # floor, then reconciles the exact old positive against its tombstone.
        join(a,b);readback=send(a,old);assert decode(readback[0])['body']['binding']['effect']=='ProtectedTombstone';assert image(b)['executions']==0
        replacement=choose(a,lambda m:tick(m)==25 and not m['anti']);send(a,replacement);b.call('run',horizon=100,budget=1);child=choose(b,lambda m:tick(m)==30 and not m['anti']);send(b,child);a.call('run',horizon=100,budget=4)
        actual=models(a,b);serial=a.call('serial');assert actual==[bytes(x) for x in serial['models']]
        receive_local(a,p40);pause=a.call('pause')['packet'];send(a,pause);pa=a.call('prepare',limit=100)['packet'];pb=b.call('prepare',limit=100)['packet'];decision=a.call('cut',prepares=[pa,pb])['packet'];send(a,decision);assert int(a.call('snapshot')['gvt'],16)==40 and int(b.call('snapshot')['gvt'],16)==40
        owner.event('scenario_pass',name='anti-before-positive',generations=[2,2]);result={'name':'anti-before-positive','models_sha256':[hashlib.sha256(x).hexdigest() for x in actual],'generations':[2,2],'floor':40};(owner.output/'anti-before-positive.json').write_text(json.dumps(result,indent=2));return result
    finally:a.stop();b.stop()
def three_member_cases(owner):
    results = []
    for name, first_peer in (("before-peer-apply", None), ("peer-1-applied", 1), ("peer-2-applied", 2)):
        start = len(owner.nodes)
        crashed = set()

        def restart(node):
            node.kill()
            crashed.add(node)
            return owner.restart(node)

        result = recover_group_in_flight(owner.workers(name), restart, first_peer)
        for node in owner.nodes[start:]:
            node.stop()
            status = node.proc.returncode
            assert status is not None and (status != 0) == (node in crashed), status
        results.append(dict(result, name=name))
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--ssh-args')
    parser.add_argument('--remote-repo', default='/content/kairos-cap48-full/kairos')
    parser.add_argument('--output', required=True)
    parser.add_argument('--cases', default='all',
                        help=f'all, reference, {GROUP_CASE}, or a comma-separated fault-stage list')
    args = parser.parse_args()
    owner = Owner(args)
    try:
        owner.setup()
        if args.cases == GROUP_CASE:
            cases = three_member_cases(owner)
            liveness = 'Fixed membership with retained in-flight work and complete successor activation'
        else:
            reference = scenario(owner, 'reference')
            results = [reference]
            choices = ['admission', 'effect', 'retirement', 'retirement-proof', 'release', 'cut', 'activation']
            if args.cases != 'all':
                choices = [] if args.cases == 'reference' else args.cases.split(',')
            for stage in choices:
                for after in (False, True):
                    result = scenario(owner, f"{stage}-{'after' if after else 'before'}", stage, after)
                    assert result['semantics'] == reference['semantics'], (
                        f'{stage}/{after}: restart changed retained model/identity/replay state'
                    )
                    results.append(result)
            if args.cases == 'all':
                results.append(anti_before_positive(owner))
            cases = [{field: result[field] for field in ('name', 'models_sha256', 'generations', 'floor')}
                     for result in results]
            liveness = 'Quiescent exact-channel cuts; missing captured obligations block prepare'
        assert owner.frozen_source=={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in owner.frozen_files};assert owner.frozen_binary==hashlib.sha256(BINARY.read_bytes()).hexdigest()
        transport = 'Native mutual-TLS gRPC'
        if owner.remote_prefix:
            transport += ' through pinned SSH forwarding'
        receipt={'passed':len(cases),'failed':0,'two_hosts':bool(owner.remote_prefix),'source':owner.frozen_source,'python':{'version':sys.version,'executable':sys.executable,'prefix':sys.prefix},'local_binary_sha256':owner.frozen_binary,'local_build_log_sha256':hashlib.sha256((owner.output/'local-build.log').read_bytes()).hexdigest(),'cases':cases,'transport':transport,'liveness':liveness}
        (owner.output/'receipt.json').write_text(json.dumps(receipt,indent=2));print(json.dumps({'CAP48_NATIVE_QUALIFICATION':receipt['passed'],'two_hosts':receipt['two_hosts'],'output':str(owner.output)}),flush=True)
    finally:owner.close()
if __name__=='__main__':main()
