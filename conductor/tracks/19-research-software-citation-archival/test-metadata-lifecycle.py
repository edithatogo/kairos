"""Exercise real metadata validator rejection paths with isolated fixture trees."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
VALIDATOR = Path(__file__).with_name('validate-citation-archive.ps1')


class MetadataLifecycle(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ['CITATION.cff', 'codemeta.json', '.zenodo.json', 'Cargo.toml',
                     'docs/research/citation.md', 'docs/research/release-metadata-status.json',
                     'paper/paper.md', 'paper/paper.bib']:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        for manifest in (ROOT / 'crates').glob('*/Cargo.toml'):
            target = self.root / manifest.relative_to(ROOT)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(manifest, target)

    def edit_json(self, name, **fields):
        path = self.root / name
        data = json.loads(path.read_text())
        data.update(fields)
        path.write_text(json.dumps(data))

    def validate(self, success):
        result = subprocess.run(['pwsh', '-NoProfile', '-File', str(VALIDATOR),
                                 '-RepoRoot', str(self.root)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0 if success else 1, result.stdout + result.stderr)

    def released(self):
        with (self.root / 'CITATION.cff').open('a') as stream:
            stream.write('date-released: "2026-10-01"\n')
        self.edit_json('codemeta.json', datePublished='2026-10-01')
        self.edit_json('.zenodo.json', publication_date='2026-10-01')
        self.edit_json('docs/research/release-metadata-status.json', status='released',
                       release_evidence={'source_commit': 'a' * 40, 'version': '0.4.0-alpha.1',
                                         'license': 'Apache-2.0 OR MIT', 'tag_url': 'https://example.test/tag',
                                         'release_url': 'https://example.test/release',
                                         'artifacts': ['fixture'], 'validation_receipts': ['fixture']})

    def test_unreleased_without_dates_passes(self):
        self.validate(True)

    def test_unreleased_publication_date_rejected(self):
        self.edit_json('codemeta.json', datePublished='2026-10-01')
        self.validate(False)

    def test_released_requires_date_and_evidence(self):
        self.edit_json('docs/research/release-metadata-status.json', status='released')
        self.validate(False)

    def test_released_consistent_fixture_passes_locally(self):
        self.released()
        self.validate(True)

    def test_released_date_disagreement_rejected(self):
        self.released()
        self.edit_json('.zenodo.json', publication_date='2026-09-01')
        self.validate(False)

    def test_crate_license_override_rejected(self):
        path = self.root / 'crates/kairo-ecs-core/Cargo.toml'
        path.write_text(path.read_text().replace('license.workspace = true', 'license = "Apache-2.0"'))
        self.validate(False)


if __name__ == '__main__':
    unittest.main()
