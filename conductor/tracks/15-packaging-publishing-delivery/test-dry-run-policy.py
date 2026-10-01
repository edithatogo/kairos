"""Isolated regression checks for the offline packaging publication boundary."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
GATE = Path('conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1')
INVENTORY = Path('packaging/release-package-manifest.json')
PUBLICATION = Path('packaging/publication-registry-manifest.json')
paths = [GATE, INVENTORY, PUBLICATION, Path('packaging/README.md'), Path('packaging/scripts/build_release_manifest.py')]
paths += [Path(m['path']) for s in json.loads((ROOT / INVENTORY).read_text())['surfaces'] for m in s['manifests']]
for case, expected in [('gated-config', 0), ('enabled-production', 1), ('unexpected-manifest', 1), ('string-health-floor', 1)]:
    with tempfile.TemporaryDirectory() as temporary:
        fixture = Path(temporary)
        for path in paths:
            destination = fixture / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / path, destination)
        (fixture / 'dist').mkdir()
        if case == 'enabled-production':
            path = fixture / PUBLICATION
            data = json.loads(path.read_text())
            data['production_publish_default'] = True
            path.write_text(json.dumps(data))
        if case == 'string-health-floor':
            path = fixture / PUBLICATION
            data = json.loads(path.read_text())
            data['health_floor'] = '9.5'
            path.write_text(json.dumps(data))
        if case == 'unexpected-manifest':
            (fixture / 'dist/publication-manifest.json').write_text('{}')
        result = subprocess.run(['pwsh', '-NoProfile', '-File', str(fixture / GATE)], capture_output=True, text=True)
        if result.returncode != expected:
            raise SystemExit(f'{case}: expected {expected}, got {result.returncode}\n{result.stdout}\n{result.stderr}')
        print(f'{case}: PASS (exit {result.returncode})')
