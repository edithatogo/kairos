#!/usr/bin/env python3
"""Native C5 process evidence; Python orchestrates the Rust executable only."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_state(root):
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=root, text=True).strip()

    return {
        'commit': git('rev-parse', 'HEAD'),
        'tree': git('rev-parse', 'HEAD^{tree}'),
        'dirty': bool(git('status', '--porcelain')),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve(strict=True)
    out = args.output.absolute()
    source_before = source_state(root)
    if source_before['dirty']:
        raise RuntimeError('qualification requires a clean source worktree')
    lock_hash = digest(root / 'Cargo.lock')
    out.mkdir(parents=True, exist_ok=False)
    study = root / 'examples/calibration/c5/study.json'
    commands = []

    def run(label, argv, expected=0):
        start = time.monotonic()
        result = subprocess.run([str(binary), *map(str, argv)], cwd=root,
                                capture_output=True, timeout=180)
        (out / f'{label}.stdout').write_bytes(result.stdout)
        (out / f'{label}.stderr').write_bytes(result.stderr)
        commands.append(dict(label=label, argv=[str(binary), *map(str, argv)],
                             cwd=str(root), exit=result.returncode,
                             elapsed_seconds=time.monotonic()-start))
        (out / 'commands.json').write_text(json.dumps(commands, indent=2)+'\n')
        if result.returncode != expected:
            raise RuntimeError(f'{label}: expected {expected}, got {result.returncode}; {result.stderr.decode()}')
        return json.loads(result.stdout) if expected == 0 else None

    for workers in (1, 2, 4):
        result = run(f'workers-{workers}', ['calibrate','--study',study,'--output',out/f'w{workers}','--workers',workers])
        assert result['schema_version'] == 'kairoecs.c5-calibration-run.v1', result
        assert result['status'] == 'complete', result
    baseline = out / 'w1'
    def compare(directory):
        expected = sorted(p.relative_to(baseline) for p in baseline.rglob('*') if p.is_file())
        actual = sorted(p.relative_to(directory) for p in directory.rglob('*') if p.is_file() and not p.name.startswith('.c5tmp-'))
        assert expected == actual, (expected, actual)
        for path in expected:
            assert (baseline/path).read_bytes() == (directory/path).read_bytes(), str(path)
    compare(out/'w2')
    compare(out/'w4')
    comparison = run('comparison',['compare-runs','--left',baseline,'--right',out/'w4'])
    assert comparison['schema_version'] == 'kairoecs.c5-run-comparison.v1', comparison
    assert comparison['compatible'] is True, comparison
    partial = run('partial',['calibrate','--study',study,'--output',out/'resume','--workers',2,'--stop-after',3])
    assert partial['status']=='incomplete', partial
    resumed = run('resume',['calibrate','--study',study,'--output',out/'resume','--resume',out/'resume','--workers',4])
    assert resumed['schema_version'] == 'kairoecs.c5-calibration-run.v1', resumed
    assert resumed['status'] == 'complete', resumed
    compare(out/'resume')

    # Kill after a complete durable shard appears, without a cooperative stop flag.
    killed = out/'killed'
    argv=[str(binary),'calibrate','--study',str(study),'--output',str(killed),'--workers','1']
    with (out/'killed.stdout').open('wb') as stdout, (out/'killed.stderr').open('wb') as stderr:
        process=subprocess.Popen(argv,cwd=root,stdout=stdout,stderr=stderr)
        deadline=time.monotonic()+120
        try:
            while not list((killed/'initial-shards').glob('task-*.json')):
                if process.poll() is not None:
                    raise RuntimeError('process exited before interrupted-work checkpoint')
                if time.monotonic()>deadline:
                    raise RuntimeError('no shard observed before kill deadline')
                time.sleep(0.002)
            process.kill()
            code=process.wait(timeout=10)
            assert code != 0
            commands.append(dict(label='hard-kill-after-durable-shard',argv=argv,cwd=str(root),exit=code))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
    killed_resume = run('killed-resume',['calibrate','--study',study,'--output',killed,'--resume',killed,'--workers',2])
    assert killed_resume['schema_version'] == 'kairoecs.c5-calibration-run.v1', killed_resume
    assert killed_resume['status'] == 'complete', killed_resume
    compare(killed)
    verification = run('verify',['verify','trace','--study',study])
    assert verification['schema_version'] == 'kairoecs.c5-trace-verification.v1', verification
    assert verification['result']['schema_version'] == 'kairoecs.calibration.trace-report.v1', verification
    assert verification['result']['status'] == 'valid', verification
    for mode in ('shadow','free'):
        replay = run(f'replay-{mode}',['replay','--study',study,'--mode',mode,'--output',out/f'replay-{mode}'])
        assert replay['schema_version'] == 'kairoecs.c5-replay-result.v1', replay
        assert replay['mode'] == mode and replay['adapter'] == 'synthetic_flow_probe.v1', replay
        assert replay['result']['schema_version'] == 'kairoecs.calibration.replay.v1', replay
        assert replay['result']['mode'] == mode, replay
    provenance=json.loads((baseline/'provenance.json').read_text())
    assert provenance['toolchain'].startswith('rustc 1.99.0 '),provenance
    assert provenance['executable_sha256']==digest(binary)
    assert provenance['source_commit'] == source_before['commit'], provenance
    assert provenance['source_dirty'] == 'false', provenance
    assert provenance['cargo_lock_sha256'] == lock_hash, provenance
    source_after = source_state(root)
    assert source_after == source_before, (source_before, source_after)
    result=dict(schema_version=1,status='pass',source_commit=source_before['commit'],
                source_tree=source_before['tree'], source_dirty=False,
                binary_sha256=digest(binary),study_sha256=digest(study),workers=[1,2,4],
                cooperative_resume='byte_identical',hard_kill_resume='byte_identical',
                summary_sha256=digest(baseline/'summary.json'),summary=json.loads((baseline/'summary.json').read_text()),
                provenance=provenance,commands=commands)
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ('status','source_commit','summary_sha256','workers','hard_kill_resume')}))


if __name__ == '__main__':
    main()
