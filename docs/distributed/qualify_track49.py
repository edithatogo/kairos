#!/usr/bin/env python3
"""Synthetic Track49 native qualification; repository-local Python3.15.2 only.

The controller owns each process/allocation and its private stores. MPI rank
death terminates the allocation; every store reopens standby before a new join.
No application retries, native capability deserialization or scalar GVT proof.
"""
from __future__ import annotations
import argparse, hashlib, importlib.util, json, os, pathlib, queue, shlex
import signal, subprocess, sys, tempfile, threading, time, uuid
if sys.version_info[:3] != (3, 15, 2):
    raise SystemExit("Use the verified repo-local Python3.15.2")
ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("cap48_receipt_helpers", ROOT / "docs/distributed/qualify_cap48.py")
old = importlib.util.module_from_spec(spec); spec.loader.exec_module(old)

def checked(args, **kw):
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          timeout=kw.pop("timeout", 90), **kw)
def decode(packet): return json.loads(bytes(packet))["record"]
def message(packet):
    b = decode(packet)["body"]
    return b.get("message") if b["kind"] == "Delivery" else None
def payload(packet):
    m = message(packet)
    return json.loads(bytes(m["payload"])) if m and not m["anti"] else None
def image(node): return json.loads(bytes(node.call("snapshot")["checkpoint"]))
def model(node): return json.loads(bytes(node.call("snapshot")["model"]))
def canonical(value): return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()

class Owner:
    def __init__(self, args):
        self.args=args;self.output=pathlib.Path(args.output).resolve();self.output.mkdir(mode=0o700,parents=True,exist_ok=True)
        self.events=(self.output/"events.jsonl").open("a");self.groups=[];self.nodes=[]
        self.binary=pathlib.Path(args.binary).resolve();self.run_id=uuid.uuid4().hex
        self.base=pathlib.Path(tempfile.mkdtemp(prefix="track49-owned-"))
        self.source={n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in self.source_paths()}
        env=dict(os.environ,CARGO_TARGET_DIR=str(self.binary.parents[2]),CARGO_PROFILE_DEV_DEBUG='0',CARGO_BUILD_JOBS='2',
                 RUSTC=checked(['which','rustc']).stdout.strip(),RUSTDOC=checked(['which','rustdoc']).stdout.strip(),RUSTUP_TOOLCHAIN='1.99.1')
        build=subprocess.run(['cargo','build','--locked','--message-format=json','-p','kairo-ecs-mpi','--example','track49_mpi_node','--features','external-fault-injection'],
            cwd=ROOT,env=env,capture_output=True,text=True,timeout=600)
        (self.output/'native-build.jsonl').write_text(build.stdout);(self.output/'native-build.log').write_text(build.stderr)
        assert build.returncode==0,'native build failed; no previous executable may qualify'
        artifacts=[json.loads(s) for s in build.stdout.splitlines() if s.startswith('{')]
        produced=[x['executable'] for x in artifacts if x.get('reason')=='compiler-artifact' and x.get('target',{}).get('name')=='track49_mpi_node' and x.get('executable')]
        assert produced and pathlib.Path(produced[-1]).resolve()==self.binary
        assert self.source=={n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in self.source}
        (self.output/'rust-version.txt').write_text(checked([env['RUSTC'],'-vV']).stdout)
        self.description=json.loads(checked([str(self.binary),'--describe']).stdout)
        self.binary_hash=hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.event("owner",base=str(self.base),binary=str(self.binary),binary_sha256=self.binary_hash,
                   source_base=checked(["git","rev-parse","HEAD"],cwd=ROOT).stdout.strip(),source=self.source)
    def source_paths(self):
        paths=checked(["git","ls-files","Cargo.toml","Cargo.lock","rust-toolchain.toml",".cargo",
            "crates/kairo-ecs-pdes","crates/kairo-ecs-mpi","crates/kairo-ecs-grpc",
            "docs/distributed/track49-fixtures","docs/distributed/qualify_track49.py"],cwd=ROOT).stdout.splitlines()
        # Include this in-progress controller before its first source checkpoint.
        return sorted(set(paths+['docs/distributed/qualify_track49.py']))
    def event(self,event,**kw):
        self.events.write(json.dumps(dict(event=event,monotonic=time.monotonic(),**kw))+"\n");self.events.flush()
    def config(self,name,n):
        return {'simulation':list(hashlib.sha256((self.run_id+":"+name).encode()).digest()[:16]),
            'model':self.description['model'],
            'members':[{'participant':r,'public_key':self.description['public_keys'][r],
                # Trusted native MPI source slot + signature binds this logical
                # transport identity; this is explicitly not a TLS certificate.
                'certificate_hash':list(hashlib.sha256(f'track49-mpi-rank-{r}'.encode()).digest()),'lps':[r]} for r in range(n)],
            'topology':[[r,[p for p in range(n) if p!=r]] for r in range(n)]}
    def group(self,name,n):
        base=self.base/name;base.mkdir(mode=0o700)
        (base/'stores').mkdir(mode=0o700)
        for r in range(n):(base/'stores'/f'node{r}').mkdir(mode=0o700)
        config=self.config(name,n);settings=base/'settings.json';settings.write_text(json.dumps({'configuration':config,'stores':str(base/'stores')}))
        (self.output/f'{name}-configuration.json').write_text(json.dumps(config,indent=2))
        g=MpiGroup(self,name,n,settings);self.groups.append(g);return g
    def restart(self,g):
        replacement=MpiGroup(self,g.name,g.n,g.settings);self.groups.append(replacement);return replacement
    def freeze_check(self):
        assert self.binary_hash==hashlib.sha256(self.binary.read_bytes()).hexdigest()
        assert self.source=={n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in self.source}
    def close(self):
        for g in self.groups:
            g.stop()
        self.events.close()

class MpiGroup:
    def __init__(self,owner,name,n,settings):
        self.owner=owner;self.name=name;self.n=n;self.settings=settings
        self.responses=queue.Queue();self.boundaries=queue.Queue();self.lock=threading.Lock()
        self.raw=(owner.output/f'{name}-{uuid.uuid4().hex[:8]}-stdout.log').open('w')
        self.stderr=(owner.output/f'{name}-{uuid.uuid4().hex[:8]}-stderr.log').open('w')
        args=['mpiexec','--timeout','180','-n',str(n),str(owner.binary),str(settings)]
        self.proc=subprocess.Popen(args,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr,text=True,bufsize=1,start_new_session=True)
        self.reader=threading.Thread(target=self._read,daemon=True);self.reader.start()
        self.ready=self.responses.get(timeout=40);assert self.ready.get('ready'),self.ready
        assert self.ready['size']==n;self.pids=[r['pid'] for r in self.ready['ranks']]
        self.owner.event('mpi_allocation_ready',name=name,size=n,ranks=self.ready['ranks'],manager=self.proc.pid)
        self.members=[MpiNode(self,r) for r in range(n)]
    def _read(self):
        for line in self.proc.stdout:
            self.raw.write(line);self.raw.flush()
            if 'STORE_BOUNDARY' in line:self.boundaries.put(line.strip());continue
            try:self.responses.put(json.loads(line))
            except json.JSONDecodeError:pass
        self.responses.put({'allocation_exited':self.proc.poll()})
    def call(self,rank,op,**kw):
        with self.lock:
            assert self.proc.poll() is None,'allocation exited'
            self.proc.stdin.write(json.dumps(dict(op=op,rank=rank,**kw),separators=(',',':'))+'\n');self.proc.stdin.flush()
            r=self.responses.get(timeout=60)
        assert 'responses' in r,r
        self.owner.event('mpi_command',name=self.name,rank=rank,operation=op,
                         outcomes=[dict(rank=i,ok=x.get('ok'),error=x.get('error')) for i,x in enumerate(r['responses'])])
        if op=='send':
            failed=next((x for x in r['responses'] if not x.get('ok')),None)
            if failed:return failed
        return r['responses'][rank]
    def kill_rank(self,rank):
        pid=self.pids[rank]
        cmd=checked(['ps','-p',str(pid),'-o','command=']).stdout
        assert 'track49_mpi_node' in cmd and str(self.settings) in cmd
        os.kill(pid,signal.SIGKILL)
        rc=self.proc.wait(timeout=20);assert rc!=0
        self.owner.event('mpi_rank_sigkill',name=self.name,rank=rank,pid=pid,manager_exit=rc)
        self._check_terminal()
    def _check_terminal(self):
        for pid in self.pids:
            p=subprocess.run(['ps','-p',str(pid),'-o','command='],capture_output=True,text=True)
            assert not ('track49_mpi_node' in p.stdout and str(self.settings) in p.stdout),'owned rank remains alive'
    def stop(self):
        if self.proc.poll() is None:
            self.proc.stdin.write('{"op":"stop"}\n');self.proc.stdin.flush();self.proc.wait(timeout=20)
        self._check_terminal();self.stderr.close();self.raw.close()

class MpiNode:
    def __init__(self,group,rank):self.group=group;self.participant=rank
    def call(self,op,expect=True,**kw):
        r=self.group.call(self.participant,op,**kw)
        if expect:assert r.get('ok'),r
        return r['result'] if r.get('ok') else r

def send(node,packet,destination=None,accept=True):
    if destination is None:destination=message(packet)['destination']
    return node.call('send',packet=packet,destination=destination,accept=accept)['replies']
def join(nodes):
    p=nodes[0].call('propose')['packet'];joins=[nodes[0].call('join',packet=p)['packet']]
    for n in nodes[1:]:joins.append(send(nodes[0],p,n.participant,False)[0])
    for n in nodes:n.call('activate',packet=p,joins=joins)
    assert all(n.call('snapshot')['active'] for n in nodes)
def pending(node):return node.call('pending')['packets']
def drain(nodes,held=frozenset()):
    for iteration in range(128):
        sent=0;used=0
        for node in nodes:
            for packet in pending(node):
                if hashlib.sha256(bytes(packet)).hexdigest() in held:continue
                send(node,packet);sent+=1
        for node in nodes:used+=node.call('run',horizon=79,budget=64)['used']
        if sent==0 and used==0:return
    raise AssertionError('bounded optimistic drain did not quiesce')
def semantic(node):
    s=image(node)
    return {k:s[k] for k in ['processes','next_incarnation','roots','known','executions','rollbacks','replays']} | {'outgoing_messages':[o['message'] for o in s['outgoing']]}
def record_nodes(owner,name,nodes):
    for n in nodes:(owner.output/f'{name}-node{n.participant}-checkpoint.json').write_text(json.dumps(image(n),indent=2))

def scenario(owner,name,n,after=None):
    group=owner.group(name,n);nodes=group.members;join(nodes)
    held=None;roots={}
    for entry in owner.description['roots']:
        nodes[0].call('root',sequence=entry['sequence'],source=0,destination=0,tick=entry['tick'],payload=entry['payload'])
        packet=next(p for p in pending(nodes[0]) if (m:=message(p)) and not m['ancestry'] and m['root_sequence']==entry['sequence'])
        roots[entry['sequence']]=packet
        if entry['sequence']==1:held=hashlib.sha256(bytes(packet)).hexdigest()
        else:send(nodes[0],packet)
    nodes[0].call('run',horizon=79,budget=64)
    transfer=next(p for p in pending(nodes[0]) if (v:=payload(p)) and v['op']=='Transfer')
    ack=send(nodes[0],transfer)
    if after is not None:
        nodes[1].call('fault',after=after);failures=[]
        def request():
            try:nodes[1].call('run',horizon=79,budget=1)
            except Exception as e:failures.append(type(e).__name__)
        thread=threading.Thread(target=request,daemon=True);thread.start()
        marker=group.boundaries.get(timeout=25);group.kill_rank(1);thread.join(timeout=1)
        owner.event('allocation_failed',name=name,boundary=marker,after=after)
        group=owner.restart(group);nodes=group.members
        assert all(not x.call('snapshot')['active'] for x in nodes)
        assert all(not x.call('run',horizon=79,budget=1,expect=False).get('ok',True) for x in nodes)
        join(nodes)
        before=nodes[1].call('snapshot')['revision'];repeated=send(nodes[0],transfer)
        assert repeated==ack and nodes[1].call('snapshot')['revision']==before,'committed admission retry changed'
    drain(nodes,{held})
    old_entity=model(nodes[1])['entities']['11']
    assert old_entity['value']==28 and [x[0] for x in old_entity['effects']]==[3,4,9]
    assert len(model(nodes[0])['acknowledgements'])==3,'ACK descendants must execute before rollback'
    owner.event('old_entity_and_descendants_applied',name=name,value=28,effects=3,acks=3)
    before=nodes[1].call('snapshot')['revision'];assert send(nodes[0],transfer)==ack
    assert nodes[1].call('snapshot')['revision']==before
    send(nodes[0],roots[1]);nodes[0].call('run',horizon=79,budget=64)
    outgoing=image(nodes[0])['outgoing']
    assert any(not o['message']['anti'] and o['message']['tick']==f'{25:032x}' and o['dependencies'] for o in outgoing)
    assert not any((m:=message(p)) and m['tick']==f'{25:032x}' for p in pending(nodes[0]))
    drain(nodes)
    actual=[bytes(x.call('snapshot')['model']) for x in nodes];reference=nodes[0].call('serial')
    assert actual==[bytes(x) for x in reference['models']],'complete native serial model/RNG/generation parity'
    final_entity=model(nodes[1])['entities']['11']
    assert final_entity['value']==128 and final_entity['generation']==7 and final_entity['component_generation']==13
    assert not final_entity['pending'] and [x[0] for x in final_entity['effects']]==[3,4,9]
    assert not model(nodes[0])['entities'] and len(model(nodes[0])['acknowledgements'])==3
    all_messages=[o['message'] for node in nodes for o in image(node)['outgoing'] if not o['message']['anti']]
    for expected in reference['messages']:
        found=next(m for m in all_messages if all(m[k]==expected[k] for k in ['source','destination','tick','payload']))
        assert all(found[k]==expected[k] for k in ['root_source','root_sequence','ancestry'])
    semantics=[semantic(x) for x in nodes];record_nodes(owner,name,nodes)
    for node in nodes:node.call('close')
    pause=nodes[0].call('pause')['packet']
    for node in nodes[1:]:send(nodes[0],pause,node.participant)
    prepares=[node.call('prepare',limit=100)['packet'] for node in nodes]
    assert not nodes[0].call('cut',prepares=prepares[:-1],expect=False).get('ok',True)
    decision=nodes[0].call('cut',prepares=prepares)['packet']
    for node in nodes[1:]:send(nodes[0],decision,node.participant)
    assert all(int(node.call('snapshot')['gvt'],16)==80 for node in nodes)
    # Equality stays retained and executable; a pre-floor root is rejected.
    record=decode(roots[6]);record['generation']=nodes[0].call('snapshot')['generation'];record['sequence']+=500
    record['body']['message'].update(tick=f'{79:032x}',root_sequence=99,incarnation=999)
    bad_packet=nodes[0].call('sign_fixture',record=record)['packet']
    before=nodes[0].call('snapshot')['revision'];bad=nodes[0].call('send',packet=bad_packet,destination=0,expect=False)
    assert not bad.get('ok',True) and nodes[0].call('snapshot')['revision']==before
    used=nodes[0].call('run',horizon=80,budget=1)['used'];assert used==1
    result={'name':name,'ranks':n,'fault_after':after,'serial_parity':True,'floor':80,
            'models_sha256':[hashlib.sha256(m).hexdigest() for m in actual],'semantics':semantics,
            'generations':[x.call('snapshot')['generation'] for x in nodes]}
    (owner.output/f'{name}.json').write_text(json.dumps(result,indent=2));owner.event('scenario_pass',name=name,ranks=n,after=after)
    group.stop();return result

def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',default=str(ROOT/'artifacts/track49/build/debug/examples/track49_mpi_node'))
    p.add_argument('--output',required=True);p.add_argument('--ranks',type=int,choices=[2,4],default=2);p.add_argument('--cases',choices=['reference','all'],default='all')
    args=p.parse_args();owner=Owner(args)
    try:
        results=[scenario(owner,'reference',args.ranks)]
        if args.cases=='all':
            for after in [False,True]:
                result=scenario(owner,'model-commit-'+('after' if after else 'before'),args.ranks,after)
                assert result['semantics']==results[0]['semantics'],'restart changed retained identity/replay/model state'
                results.append(result)
        owner.freeze_check();receipt={'passed':len(results),'failed':0,'ranks':args.ranks,'source':owner.source,'binary_sha256':owner.binary_hash,
            'cases':[{k:v for k,v in x.items() if k!='semantics'} for x in results],
            'runtime':'actual native MPI records, strict signed driver and real SQL process death; whole-allocation restart',
            'python':{'version':sys.version,'executable':sys.executable,'prefix':sys.prefix}}
        (owner.output/'receipt.json').write_text(json.dumps(receipt,indent=2));print(json.dumps({'TRACK49_MPI_PASSED':len(results),'ranks':args.ranks}),flush=True)
    finally:owner.close()
if __name__=='__main__':main()
