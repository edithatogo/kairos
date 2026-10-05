#!/usr/bin/env python3
"""Run the bounded native C2.0 paired-preemption fixture in an isolated archive."""
from __future__ import annotations
import argparse, hashlib, io, json, os, re, shutil, subprocess, sys, tarfile
from collections import Counter
from pathlib import Path

PACKET = Path('.artifacts/packets/C2.0.red-tests.paired-preemption.json')
ARTIFACT = Path('.artifacts/mvp/C2.0.red-tests.paired-preemption')
FIXTURE = Path('conformance/c20/preemption_flow_c20.rs')
BASE = 'd20ec0356e3daa6973c0f382b079076af05cd4c3'
TOOLCHAIN = Path('/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin')
TESTS = {
 'preemption_flow_c20::tests::suspend_preemption_resumes_remaining_work_and_preserves_completed_micro_transit',
 'preemption_flow_c20::tests::restart_preemption_rebuilds_original_template_without_resampling_or_transit_reset',
}
def sha(b: bytes) -> str: return hashlib.sha256(b).hexdigest()
def run(argv, cwd, env):
 return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
def main():
 p=argparse.ArgumentParser(); p.add_argument('--expect',choices=('red','green'),default='red'); mode=p.parse_args().expect
 root=Path(__file__).resolve().parents[2]; packet=json.loads((root/PACKET).read_text())
 if packet['base_commit'] != BASE: raise RuntimeError('frozen packet base changed')
 for name,expected in packet['input_hashes'].items():
  if sha((root/name).read_bytes()) != expected: raise RuntimeError(f'frozen input hash changed: {name}')
 head=run(['git','rev-parse','HEAD'],root,os.environ.copy())
 if head.returncode or head.stdout.strip()!=BASE: raise RuntimeError('runner requires exact reviewed packet base')
 status=run(['git','status','--porcelain','--untracked-files=no'],root,os.environ.copy())
 if status.returncode or status.stdout.strip(): raise RuntimeError('runner requires committed tracked source changes')
 fixture=(root/FIXTURE).read_bytes(); ar=subprocess.run(['git','archive','--format=tar','HEAD'],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=False)
 if ar.returncode: raise RuntimeError('git archive failed')
 art=root/ARTIFACT; logs=art/'logs'; disp=art/'disposable'; logs.mkdir(parents=True,exist_ok=True)
 if disp.exists(): shutil.rmtree(disp)
 disp.mkdir(parents=True)
 with tarfile.open(fileobj=io.BytesIO(ar.stdout),mode='r:') as tf: tf.extractall(disp,filter='data')
 crate=disp/'crates/kairo-ecs-calibration/src'; (crate/'preemption_flow_c20.rs').write_bytes(fixture)
 lib=crate/'lib.rs'; text=lib.read_text(); attach='\n#[cfg(test)]\n#[path = "preemption_flow_c20.rs"]\nmod preemption_flow_c20;\n'
 if mode=='red':
  attach += '#[path = "flow_bridge.rs"]\nmod flow_bridge;\n'
 lib.write_text(text+attach)
 env=os.environ.copy(); env['PATH']=f'{TOOLCHAIN}:/opt/homebrew/bin:/usr/bin:/bin:/Users/doughnut/.cargo/bin'; env['RUSTC']=str(TOOLCHAIN/'rustc'); env['RUSTDOC']=str(TOOLCHAIN/'rustdoc'); env['CARGO_TARGET_DIR']=str(art/'target')
 argv=['rustup','run','1.99.0','cargo','test','--locked','-p','kairo-ecs-calibration']
 argv += ['--lib','preemption_flow_c20::tests::','--','--nocapture'] if mode=='red' else ['--features','flow','--lib','preemption_flow_c20::tests::','--','--nocapture']
 result=run(argv,disp,env); log=logs/('cargo-native-red.log' if mode=='red' else 'cargo-native-green.log'); log.write_text(result.stdout)
 diagnostics=re.findall(r"error: couldn't (?:find|read) file `?[^`\n]+\.rs`?",result.stdout)
 missing={}
 for name in ('work_duration.rs','flow_bridge.rs'):
  try: (crate/name).stat(); enoent=None
  except FileNotFoundError as exc: enoent=exc.errno
  missing[name]={'errno':enoent,'diagnostics':[d for d in diagnostics if name in d]}
 errors=[line for line in re.findall(r'^error(?:\[[^]]+\])?: .+$',result.stdout,re.M) if not line.startswith('error: could not compile ')]
 red=(result.returncode==101 and diagnostics==["error: couldn't find file `crates/kairo-ecs-calibration/src/flow_bridge.rs`"] and errors==diagnostics and missing['flow_bridge.rs']['errno']==2 and len(missing['flow_bridge.rs']['diagnostics'])==1 and missing['work_duration.rs']['errno']==2)
 observed=re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)$',result.stdout,re.M); counts=Counter(n for n,_ in observed)
 summary=re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;',result.stdout)
 green=(result.returncode==0 and summary is not None and all(counts[n]==1 and (n,'ok') in observed for n in TESTS) and all(s=='ok' for _,s in observed))
 verified=red if mode=='red' else green
 receipt={'packet_id':'C2.0.red-tests.paired-preemption','task_status':'ready_for_review' if verified else 'blocked','expectation':mode,'green_expectation_unverified':mode=='red','claim':'exact missing production module red preparation only; no behavioral acceptance' if mode=='red' else 'named actual Flow/provider preemption tests passed; not C2.0/C2.1 acceptance','packet_base':BASE,'committed_workspace':head.stdout.strip(),'fixture':str(FIXTURE),'fixture_sha256':sha(fixture),'command':{'argv':argv,'cwd':str(disp),'exit_status':result.returncode,'expected_exit_status':101 if mode=='red' else 0,'missing_modules':missing if mode=='red' else {},'compiler_error_lines':errors if mode=='red' else [],'expected_tests':sorted(TESTS) if mode=='green' else [],'observed_tests':[{'name':n,'status':s} for n,s in observed],'summary':summary.group(0) if summary else None,'green_verified':green if mode=='green' else None,'log':str(log.relative_to(root)),'log_sha256':sha(result.stdout.encode())},'toolchain':{'version':'1.99.0','rustc':str(TOOLCHAIN/'rustc'),'rustdoc':str(TOOLCHAIN/'rustdoc')},'limitations':['Disposable git archive only; no live Rust source, Cargo manifest or lockfile changed.','Expected red proves parser/module wiring only, not behavior.','Full Flow checkpoint and paired/transit C2.0/C2.1 gates remain open.']}
 rp=art/'result.json'; rp.write_text(json.dumps(receipt,indent=2,sort_keys=True)+'\n'); print(json.dumps({'task_status':receipt['task_status'],'exit':result.returncode,'expected_red':red,'green_verified':green,'fixture_sha256':receipt['fixture_sha256'],'log':receipt['command']['log'],'result':str(rp.relative_to(root))},sort_keys=True)); return 0 if verified else 1
if __name__=='__main__':
 try: raise SystemExit(main())
 except Exception as exc: print(f'runner error: {exc}',file=sys.stderr); raise SystemExit(2)
