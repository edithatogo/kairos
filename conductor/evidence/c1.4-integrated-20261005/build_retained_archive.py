from pathlib import Path
import hashlib
import io
import json
import os
import re
import subprocess
import tarfile
import time

root = Path('/private/tmp/kairos-c14-integrated-20261005')
destination = root / 'conductor/evidence/c1.4-integrated-20261005'
destination.mkdir(parents=True, exist_ok=True)
sources = {
    'runtime': Path('/private/tmp/careops-c14-ci-20261005/libs/kairos/.artifacts/c14-root'),
    'independent': Path('/private/tmp/careops-c14-ci-policy-pin-20261005/libs/kairos/.artifacts/c14-independent'),
    'calibration-worker': Path('/private/tmp/careops-c14-calibration-20261005/libs/kairos/.artifacts/c14-calibration'),
    'inventory-worker': Path('/private/tmp/careops-c14-inventory-20261005/libs/kairos/.artifacts/c14-inventory'),
    'ci-policy-worker': Path('/private/tmp/careops-c14-ci-policy-pin-20261005/libs/kairos/.artifacts/c14-ci-policy-pin'),
    'ci-core-worker': Path('/private/tmp/careops-c14-calibration-20261005/libs/kairos/.artifacts/c14-ci-core-env'),
    'doc-worker': Path('/private/tmp/careops-c14-doc-link-20261005/.artifacts/c14-doc-link'),
    'mvp-packets': Path('/private/tmp/careops-c14-ci-20261005/libs/kairos/.artifacts/mvp'),
    'governance-preflight': root / '.artifacts/c14-governance',
}
skip_dirs = {'__pycache__', '.git', 'node_modules', '.astro', 'build'}
members = {}
excluded = []
for label, directory in sources.items():
    assert directory.is_dir(), directory
    for parent, dirs, files in os.walk(directory):
        dirs[:] = [d for d in dirs if d not in skip_dirs and not d.startswith(('target', 'venv'))]
        if label == 'mvp-packets':
            dirs[:] = [d for d in dirs if d.startswith('C1.4.')]
        for filename in files:
            path = Path(parent) / filename
            relative = path.relative_to(directory)
            if any(word in filename.lower() for word in ('claim', 'lease', 'token')):
                excluded.append(label + '/' + str(relative))
                continue
            assert path.is_file() and not path.is_symlink(), path
            data = path.read_bytes()
            # Operational bearer lease data is never part of the public proof.
            if re.search(rb'"token"\s*:\s*"[0-9a-f]{32}"', data):
                excluded.append(label + '/' + str(relative))
                continue
            members[label + '/' + str(relative)] = data
for name in ('readback-accepted-candidate.json', 'reconciled-counts-accepted-candidate.json', 'execution-receipt-accepted-candidate.json'):
    assert 'independent/' + name in members, name
for scope in ['crates/kairo-ecs-calibration', 'crates/kairo-ecs-arrow-io', 'crates/kairo-ecs-arrow', 'crates/kairo-ecs-core', 'crates/kairo-ecs-types', 'crates/kairo-ecs-state', 'crates/kairo-ecs-rng', 'Cargo.toml', 'Cargo.lock', 'conformance']:
    names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', '7f72b7d', '--', scope], cwd=root, text=True).splitlines()
    for name in names:
        data = subprocess.check_output(['git', 'show', '7f72b7d:' + name], cwd=root)
        assert data == (root / name).read_bytes(), name
        members['source-runtime/' + name] = data
archive = destination / 'qualification.tar.gz'
assert not archive.exists()
started = time.time()
with tarfile.open(archive, 'w:gz', compresslevel=9) as stream:
    for name, data in sorted(members.items()):
        item = tarfile.TarInfo(name)
        item.size = len(data)
        item.mode = 0o644
        item.mtime = 0
        stream.addfile(item, io.BytesIO(data))
inventory = {'schema_version': 'c14.retained-evidence-inventory.v1', 'members': {
    name: {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
    for name, data in sorted(members.items())}}
(destination / 'artifact-inventory.json').write_text(json.dumps(inventory, indent=2) + '\n')
with tarfile.open(archive, 'r:gz') as stream:
    actual = stream.getmembers()
    assert len(actual) == len(members)
    assert {item.name for item in actual} == set(members)
    for item in actual:
        assert item.isfile()
        data = stream.extractfile(item).read()
        assert len(data) == inventory['members'][item.name]['bytes']
        assert hashlib.sha256(data).hexdigest() == inventory['members'][item.name]['sha256']
sha = hashlib.sha256(archive.read_bytes()).hexdigest()
(destination / 'SHA256SUMS').write_text(sha + '  qualification.tar.gz\n')
(destination / 'archive-build-receipt.json').write_text(json.dumps({
    'argv': ['python3', str(Path(__file__).resolve())], 'cwd': str(root),
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
    'start': started, 'end': time.time(), 'exit_code': 0,
    'runtime_tested_source': '7f72b7d9a9b6ae62e481a59b2cc04ce37da0a4a5',
    'accepted_Q5_2_source': '8daa0978578b8a5b5b6427e84db1a3e6c54a1123',
    'archive_sha256': sha, 'archive_bytes': archive.stat().st_size,
    'members': len(members), 'all_member_hashes_verified': True,
    'excluded_operational_files': excluded,
    'cache_directories_excluded': sorted(skip_dirs) + ['target*', 'venv*'],
}, indent=2) + '\n')
(destination / 'build_retained_archive.py').write_bytes(Path(__file__).read_bytes())
print('Retained verified archive', len(members), archive.stat().st_size, sha)
