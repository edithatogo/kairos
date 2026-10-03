import copy
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('acquisition', Path(__file__).resolve().parents[1] / 'packaging/scripts/acquire_package_archive_bundle.py')
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)

class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.sha = '1' * 40
        self.run = {'id': 7, 'head_sha': self.sha, 'repository': {'full_name': a.REPOSITORY}, 'head_repository': {'full_name': a.REPOSITORY}, 'path': a.WORKFLOW, 'status': 'completed', 'conclusion': 'success'}
        self.artifact = {'id': 9, 'name': 'kairos-actual-package-archives-' + self.sha, 'digest': 'sha256:' + 'a' * 64, 'expired': False}
        self.inventory = {'total_count': 1, 'artifacts': [self.artifact]}

    def test_exact_run_and_archive_identity(self):
        self.assertEqual(a.select_artifact(self.run, self.inventory, 7, self.sha), self.artifact)

    def test_wrong_run_sha_workflow_status_and_repository(self):
        for key, value in [('id', 8), ('head_sha', '2' * 40), ('path', 'other.yml'), ('status', 'in_progress'), ('conclusion', 'failure'), ('repository', {'full_name': 'elsewhere/repo'}), ('head_repository', {'full_name': 'fork/kairos'})]:
            with self.subTest(key=key):
                run = copy.deepcopy(self.run)
                run[key] = value
                with self.assertRaises(ValueError):
                    a.select_artifact(run, self.inventory, 7, self.sha)

    def test_expired_missing_ambiguous_and_incomplete_inventory(self):
        inventories = [{'total_count': 0, 'artifacts': []}, {'total_count': 2, 'artifacts': [self.artifact, self.artifact]}, {'total_count': 2, 'artifacts': [self.artifact]}]
        for key, value in [('expired', True), ('digest', None), ('id', True)]:
            item = dict(self.artifact, **{key: value})
            inventories.append({'total_count': 1, 'artifacts': [item]})
        for inv in inventories:
            with self.assertRaises(ValueError):
                a.select_artifact(self.run, inv, 7, self.sha)

    def test_verified_safe_extraction_and_digest_tamper(self):
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            archive = root / 'a.zip'
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr('rust/a.crate', b'payload')
            digest = 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest()
            a.extract_verified(archive, root / 'ok', digest)
            self.assertEqual((root / 'ok/rust/a.crate').read_bytes(), b'payload')
            with self.assertRaises(ValueError):
                a.extract_verified(archive, root / 'bad', 'sha256:' + '0' * 64)
            self.assertFalse((root / 'bad').exists())

    def test_unsafe_paths_and_symlink_rejected_before_output(self):
        for name in ['../escape', '/absolute', 'a/../escape', 'a//b', 'a/./b', 'a\\b', 'C:drive']:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as t:
                root = Path(t)
                archive = root / 'a.zip'
                with zipfile.ZipFile(archive, 'w') as z:
                    z.writestr(name, b'bad')
                digest = 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest()
                with self.assertRaises(ValueError):
                    a.extract_verified(archive, root / 'output', digest)
                self.assertFalse((root / 'output').exists())
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            archive = root / 'a.zip'
            info = zipfile.ZipInfo('link')
            info.external_attr = 0o120777 << 16
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr(info, b'outside')
            with self.assertRaises(ValueError):
                a.extract_verified(archive, root / 'output', 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest())

    def test_pr_build_merge_and_head_are_distinct_verified_identities(self):
        head = '2' * 40
        base = '3' * 40
        run = dict(self.run, event='pull_request', head_sha=head, pull_requests=[{'head': {'sha': head}, 'base': {'sha': base}}])
        commit = {'sha': self.sha, 'parents': [{'sha': base}, {'sha': head}]}
        self.assertEqual(a.select_artifact(run, self.inventory, 7, self.sha, head, commit), self.artifact)
        for bad in [None, {'sha': self.sha, 'parents': [{'sha': head}, {'sha': base}]}, {'sha': '4' * 40, 'parents': commit['parents']}]:
            with self.assertRaises(ValueError):
                a.select_artifact(run, self.inventory, 7, self.sha, head, bad)
        with self.assertRaises(ValueError):
            a.select_artifact(run, self.inventory, 7, self.sha)

    def test_full_acquisition_preserves_seven_ecosystem_bundle(self):
        import io
        import json
        import tarfile
        from unittest.mock import patch
        bundle_spec = importlib.util.spec_from_file_location('bundle_test', Path(a.__file__).with_name('build_package_archive_bundle.py'))
        bundle = importlib.util.module_from_spec(bundle_spec)
        bundle_spec.loader.exec_module(bundle)
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            source = root / 'source'
            for ecosystem, (_, extensions) in bundle.ARCHIVES.items():
                directory = source / ecosystem
                directory.mkdir(parents=True)
                (directory / 'BUILD-INFO.json').write_text(json.dumps({'ecosystem': ecosystem, 'source_commit': self.sha, 'command': 'synthetic build', 'toolchain': 'fixture', 'platform': 'fixture', 'exit_status': 0}))
                suffix = sorted(extensions)[0]
                archive = directory / ('sample' + suffix)
                if suffix in {'.whl', '.nupkg'}:
                    with zipfile.ZipFile(archive, 'w') as z:
                        z.writestr('package/payload', b'payload')
                else:
                    with tarfile.open(archive, 'w:gz') as z:
                        info = tarfile.TarInfo('package/payload')
                        info.size = 7
                        z.addfile(info, io.BytesIO(b'payload'))
            retained = root / 'retained'
            bundle.build(source, retained, self.sha)
            archive = root / 'download.zip'
            with zipfile.ZipFile(archive, 'w') as z:
                for item in retained.rglob('*'):
                    if item.is_file():
                        z.write(item, item.relative_to(retained).as_posix())
            self.artifact['digest'] = 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest()
            payload = archive.read_bytes()
            def download(argv, path):
                self.assertEqual(argv[-1], 'repos/edithatogo/kairos/actions/artifacts/9/zip')
                path.write_bytes(payload)
                return self.artifact['digest']
            output = root / 'acquired'
            with patch.object(a, 'api', side_effect=[self.run, self.inventory]), patch.object(a, 'download_verified', side_effect=download), patch('sys.argv', ['acquire', '--run-id', '7', '--source-commit', self.sha, '--output', str(output)]):
                a.main()
            bundle.verify(output)
            self.assertEqual({p.relative_to(output).as_posix(): p.read_bytes() for p in output.rglob('*') if p.is_file()}, {p.relative_to(retained).as_posix(): p.read_bytes() for p in retained.rglob('*') if p.is_file()})
            receipt = json.loads(output.with_name('acquired.acquisition.json').read_text())
            self.assertEqual(receipt['source_commit'], self.sha)
            self.assertEqual(receipt['artifact_digest'], self.artifact['digest'])


    def test_streamed_download_exact_limit_and_existing_destination(self):
        import sys
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'download.zip'
            digest = a.download_verified([sys.executable, '-c', 'import sys;sys.stdout.buffer.write(b"x"*32)'], path, 32)
            self.assertEqual(path.read_bytes(), b'x' * 32)
            self.assertEqual(digest, 'sha256:' + hashlib.sha256(b'x' * 32).hexdigest())
            with self.assertRaises(FileExistsError):
                a.download_verified([sys.executable, '-c', 'raise SystemExit(99)'], path, 32)
            self.assertEqual(path.read_bytes(), b'x' * 32)

    def test_streamed_download_over_limit_and_child_failure_cleanup(self):
        import sys
        import subprocess
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'download.zip'
            with self.assertRaisesRegex(ValueError, 'byte limit'):
                a.download_verified([sys.executable, '-c', 'import sys;sys.stdout.buffer.write(b"x"*33)'], path, 32)
            self.assertFalse(path.exists())
            with self.assertRaises(subprocess.CalledProcessError):
                a.download_verified([sys.executable, '-c', 'import sys;sys.stdout.buffer.write(b"partial");sys.exit(7)'], path, 32)
            self.assertFalse(path.exists())

    def test_extraction_hashes_without_whole_archive_read(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            archive = root / 'a.zip'
            with zipfile.ZipFile(archive, 'w') as z:
                z.writestr('payload', b'content')
            digest = 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest()
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('whole archive allocation')):
                a.extract_verified(archive, root / 'output', digest)
            self.assertEqual((root / 'output/payload').read_bytes(), b'content')


    def test_live_over_limit_producer_is_killed_and_reaped(self):
        import sys
        from unittest.mock import patch
        real_popen = a.subprocess.Popen
        children = []
        def start(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            children.append(child)
            return child
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'download.zip'
            with patch.object(a.subprocess, 'Popen', side_effect=start):
                with self.assertRaisesRegex(ValueError, 'byte limit'):
                    a.download_verified([sys.executable, '-c', 'import sys,time;sys.stdout.buffer.write(b"x"*33);sys.stdout.buffer.flush();time.sleep(30)'], path, 32)
            self.assertEqual(len(children), 1)
            self.assertIsNotNone(children[0].returncode)
            self.assertIsNotNone(children[0].poll())
            self.assertFalse(path.exists())

    def test_producer_start_failure_removes_only_new_download(self):
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'download.zip'
            with self.assertRaises(FileNotFoundError):
                a.download_verified([str(Path(t) / 'missing-executable')], path, 32)
            self.assertFalse(path.exists())

    def test_multichunk_exact_limit_has_matching_digest(self):
        import sys
        size = 1024 * 1024 + 17
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'download.zip'
            digest = a.download_verified([sys.executable, '-c', f'import sys;sys.stdout.buffer.write(b"z"*{size})'], path, size)
            self.assertEqual(path.stat().st_size, size)
            self.assertEqual(digest, 'sha256:' + hashlib.sha256(b'z' * size).hexdigest())

if __name__ == '__main__':
    unittest.main()
