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
        self.run = {'id': 7, 'head_sha': self.sha, 'repository': {'full_name': a.REPOSITORY, 'id': 123}, 'head_repository': {'full_name': a.REPOSITORY, 'id': 123}, 'path': a.WORKFLOW, 'status': 'completed', 'conclusion': 'success'}
        self.artifact = {'id': 9, 'name': 'kairos-actual-package-archives-' + self.sha, 'digest': 'sha256:' + 'a' * 64, 'expired': False, 'workflow_run': {'id': 7, 'head_sha': self.sha, 'repository_id': 123, 'head_repository_id': 123}}
        self.inventory = {'total_count': 1, 'artifacts': [self.artifact]}

    def make_archive_zip(self, root, source_commit):
        import io
        import json
        import tarfile
        bundle_spec = importlib.util.spec_from_file_location('retained_bundle_test', Path(a.__file__).with_name('build_package_archive_bundle.py'))
        bundle = importlib.util.module_from_spec(bundle_spec)
        bundle_spec.loader.exec_module(bundle)
        source = root / ('source-' + source_commit[:6])
        for ecosystem, (_, extensions) in bundle.ARCHIVES.items():
            directory = source / ecosystem
            directory.mkdir(parents=True)
            (directory / 'BUILD-INFO.json').write_text(json.dumps({'ecosystem': ecosystem, 'source_commit': source_commit, 'command': 'synthetic build', 'toolchain': 'fixture', 'platform': 'fixture', 'exit_status': 0}))
            suffixes = sorted(extensions) if ecosystem == 'python' else [sorted(extensions)[0]]
            for suffix in suffixes:
                archive = directory / ('sample' + suffix)
                if suffix in {'.whl', '.nupkg'}:
                    with zipfile.ZipFile(archive, 'w') as z:
                        z.writestr('package/payload', b'payload')
                else:
                    with tarfile.open(archive, 'w:gz') as z:
                        info = tarfile.TarInfo('package/payload')
                        info.size = 7
                        z.addfile(info, io.BytesIO(b'payload'))
        retained = root / ('retained-' + source_commit[:6])
        bundle.build(source, retained, source_commit)
        archive_zip = root / ('producer-' + source_commit[:6] + '.zip')
        with zipfile.ZipFile(archive_zip, 'w') as z:
            for item in retained.rglob('*'):
                if item.is_file():
                    z.write(item, item.relative_to(retained).as_posix())
        payload = archive_zip.read_bytes()
        return bundle, payload, 'sha256:' + hashlib.sha256(payload).hexdigest()

    def source_readback(self, source, tree, parents):
        return {
            'sha': source,
            'url': f'https://api.github.com/repos/{a.REPOSITORY}/git/commits/{source}',
            'html_url': f'https://github.com/{a.REPOSITORY}/commit/{source}',
            'tree': {'sha': tree, 'url': f'https://api.github.com/repos/{a.REPOSITORY}/git/trees/{tree}'},
            'verification': {'verified': True, 'reason': 'valid', 'signature': 'fixture-signature', 'payload': 'fixture-payload'},
            'parents': [{'sha': parent, 'url': f'https://api.github.com/repos/{a.REPOSITORY}/git/commits/{parent}', 'html_url': f'https://github.com/{a.REPOSITORY}/commit/{parent}', 'node_id': 'fixture-node'} for parent in parents],
        }

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
        self.artifact['workflow_run']['head_sha'] = head
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

    def test_retained_main_acquisition_keeps_zip_and_all_admission_readbacks(self):
        import json
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            bundle, payload, digest = self.make_archive_zip(root, self.sha)
            source_tree = '3' * 40
            prior = '4' * 40
            commit = self.source_readback(self.sha, source_tree, [prior])
            run = dict(self.run, event='workflow_dispatch', head_branch='main', pull_requests=[])
            artifact = copy.deepcopy(self.artifact)
            artifact['digest'] = digest
            artifact['workflow_run']['head_branch'] = 'main'
            inventory = {'total_count': 1, 'artifacts': [artifact]}
            observed_main = '2' * 40
            branch = {'name': 'main', 'commit': {'sha': observed_main}}
            compare_path = f'repos/{a.REPOSITORY}/compare/{self.sha}...{observed_main}'
            compare = {'url': f'https://api.github.com/{compare_path}', 'base_commit': {'sha': self.sha}, 'merge_base_commit': {'sha': self.sha}, 'status': 'ahead', 'ahead_by': 1, 'behind_by': 0}

            def acquire(output, branch_readback, compare_readback):
                rows = {
                    f'repos/{a.REPOSITORY}/actions/runs/7': run,
                    f'repos/{a.REPOSITORY}/branches/main': branch_readback,
                    f'repos/{a.REPOSITORY}/actions/runs/7/artifacts?per_page=100': inventory,
                    f'repos/{a.REPOSITORY}/git/commits/{self.sha}': commit,
                }
                if compare_readback is not None:
                    rows[compare_path] = compare_readback
                def fake_api(path):
                    return rows[path]
                def fake_download(argv, path):
                    self.assertEqual(argv[-1], f'repos/{a.REPOSITORY}/actions/artifacts/9/zip')
                    path.write_bytes(payload)
                    return digest
                argv = ['acquire', '--run-id', '7', '--require-main-dispatch', '--acquisition-output', str(output)]
                with patch.object(a, 'api', side_effect=fake_api), patch.object(a, 'download_verified', side_effect=fake_download) as download_mock, patch('sys.argv', argv):
                    a.main()
                download_mock.assert_called_once()

            output = root / 'acquisition-main'
            acquire(output, branch, compare)
            bundle.verify(output / 'bundle')
            self.assertEqual((output / '9.zip').read_bytes(), payload)
            receipt = json.loads((output / 'acquisition.json').read_text())
            self.assertEqual(receipt['source_commit'], self.sha)
            self.assertEqual(receipt['archive_index_sha256'], hashlib.sha256((output / 'bundle/ARCHIVE-INDEX.json').read_bytes()).hexdigest())
            self.assertEqual(receipt['main_ancestry']['observed_main_sha'], observed_main)
            self.assertEqual(receipt['scope'], 'verified acquisition; not original build provenance or release acceptance')
            self.assertEqual(receipt['build_commit_parents'], commit['parents'])
            self.assertEqual(receipt['archive_index_sha256'], hashlib.sha256((output / 'bundle/ARCHIVE-INDEX.json').read_bytes()).hexdigest())
            self.assertEqual(json.loads((output / 'receipt.json').read_text())['producer_pr_head'], self.sha)
            self.assertEqual(json.loads((output / 'receipt.json').read_text())['selection_policy'], 'same-repository-main-workflow-dispatch')
            self.assertEqual(json.loads((output / 'receipt.json').read_text())['actual_archive_count'], 8)
            self.assertEqual(json.loads((output / 'artifact-metadata.json').read_text()), artifact)
            self.assertEqual(json.loads((output / 'run-metadata.json').read_text()), run)
            self.assertEqual(json.loads((output / 'branch-main-readback.json').read_text()), branch)
            self.assertEqual(json.loads((output / 'compare-main-readback.json').read_text()), compare)
            self.assertEqual(json.loads((output / 'source-commit-readback.json').read_text())[self.sha], a.validate_source_commit_readback(commit, self.sha, self.sha))
            self.assertEqual(json.loads((output / 'source-commit-api-readback.json').read_text())[self.sha], commit)
            self.assertEqual(set(json.loads((output / 'source-commit-readback.json').read_text())[self.sha]['parents'][0]), {'sha', 'url', 'html_url'})
            self.assertEqual(json.loads((output / 'source-commit-api-readback.json').read_text())[self.sha]['parents'][0]['node_id'], 'fixture-node')

            identity_output = root / 'acquisition-identical-main'
            acquire(identity_output, {'name': 'main', 'commit': {'sha': self.sha}}, None)
            self.assertEqual(json.loads((identity_output / 'acquisition.json').read_text())['main_ancestry']['status'], 'identical')
            self.assertFalse((identity_output / 'compare-main-readback.json').exists())

    def test_retained_pr_acquisition_requires_distinct_head_parent(self):
        import json
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            source = self.sha
            head = '2' * 40
            base = '3' * 40
            bundle, payload, digest = self.make_archive_zip(root, source)
            run = dict(self.run, event='pull_request', head_sha=head, pull_requests=[{'head': {'sha': head}, 'base': {'sha': base}}])
            artifact = copy.deepcopy(self.artifact)
            artifact['digest'] = digest
            artifact['workflow_run']['head_sha'] = head
            inventory = {'total_count': 1, 'artifacts': [artifact]}
            commit = self.source_readback(source, '4' * 40, [base, head])
            commit_api = {'sha': source, 'commit': {'tree': commit['tree'], 'verification': commit['verification']}, 'parents': commit['parents']}
            rows = {
                f'repos/{a.REPOSITORY}/actions/runs/7': run,
                f'repos/{a.REPOSITORY}/actions/runs/7/artifacts?per_page=100': inventory,
                f'repos/{a.REPOSITORY}/commits/{source}': commit_api,
                f'repos/{a.REPOSITORY}/git/commits/{source}': commit,
            }
            def fake_download(argv, path):
                path.write_bytes(payload)
                return digest
            output = root / 'acquisition-pr'
            argv = ['acquire', '--run-id', '7', '--source-commit', source, '--head-commit', head, '--acquisition-output', str(output)]
            with patch.object(a, 'api', side_effect=lambda path: rows[path]), patch.object(a, 'download_verified', side_effect=fake_download), patch('sys.argv', argv):
                a.main()
            bundle.verify(output / 'bundle')
            receipt = json.loads((output / 'receipt.json').read_text())
            self.assertEqual(receipt['producer_pr_head'], head)
            self.assertEqual(receipt['actual_archive_count'], 8)
            self.assertNotEqual(receipt['producer_checkout_source'], head)
            self.assertEqual((output / '9.zip').read_bytes(), payload)

    def test_source_commit_readback_rejects_missing_identity_parent_and_signature(self):
        tree = '3' * 40
        parent = '4' * 40
        valid = self.source_readback(self.sha, tree, [parent])
        self.assertEqual(a.validate_source_commit_readback(valid, self.sha, self.sha)['tree']['sha'], tree)
        invalid = [None, dict(valid, sha='2' * 40), self.source_readback(self.sha, 'bad-tree', [parent]),
                   self.source_readback(self.sha, tree, []), self.source_readback(self.sha, tree, [dict(valid['parents'][0], url='https://wrong')]),
                   dict(valid, verification={'verified': False, 'reason': 'unsigned'}),
                   self.source_readback(self.sha, tree, [self.sha]),
                   self.source_readback(self.sha, tree, [parent, parent])]
        for bad in invalid:
            with self.subTest(readback=bad), self.assertRaises(ValueError):
                a.validate_source_commit_readback(bad, self.sha, self.sha)
        with self.assertRaisesRegex(ValueError, 'PR producer head'):
            a.validate_source_commit_readback(valid, self.sha, '5' * 40)

    def test_retained_output_rejects_existing_root_and_cleans_failed_fresh_root(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            existing = root / 'existing'
            existing.mkdir()
            sentinel = existing / 'keep.txt'
            sentinel.write_text('preserve')
            with patch.object(a, 'api') as api_mock, patch('sys.argv', ['acquire', '--run-id', '7', '--acquisition-output', str(existing)]):
                with self.assertRaises(SystemExit):
                    a.main()
            api_mock.assert_not_called()
            self.assertEqual(sentinel.read_text(), 'preserve')

            source_tree = '3' * 40
            parent = '4' * 40
            run = dict(self.run, event='workflow_dispatch', head_branch='main', pull_requests=[])
            artifact = copy.deepcopy(self.artifact)
            artifact['workflow_run']['head_branch'] = 'main'
            branch = {'name': 'main', 'commit': {'sha': self.sha}}
            commit = self.source_readback(self.sha, source_tree, [parent])
            inventory = {'total_count': 1, 'artifacts': [artifact]}
            rows = {
                f'repos/{a.REPOSITORY}/actions/runs/7': run,
                f'repos/{a.REPOSITORY}/branches/main': branch,
                f'repos/{a.REPOSITORY}/actions/runs/7/artifacts?per_page=100': inventory,
                f'repos/{a.REPOSITORY}/git/commits/{self.sha}': commit,
            }
            for name, fake_download in [
                ('digest-mismatch', lambda argv, path: (path.write_bytes(b'partial'), 'sha256:' + '0' * 64)[1]),
                ('partial-error', lambda argv, path: (path.write_bytes(b'partial'), (_ for _ in ()).throw(OSError('fixture download failure')))[1]),
            ]:
                output = root / name
                argv = ['acquire', '--run-id', '7', '--require-main-dispatch', '--acquisition-output', str(output)]
                with self.subTest(name=name), patch.object(a, 'api', side_effect=lambda path: rows[path]), patch.object(a, 'download_verified', side_effect=fake_download), patch('sys.argv', argv):
                    with self.assertRaises((ValueError, OSError)):
                        a.main()
                self.assertFalse(output.exists())
                self.assertEqual(sentinel.read_text(), 'preserve')

            missing_readback_output = root / 'missing-readback'
            missing_rows = {key: value for key, value in rows.items() if not key.endswith('/git/commits/' + self.sha)}
            argv = ['acquire', '--run-id', '7', '--require-main-dispatch', '--acquisition-output', str(missing_readback_output)]
            with patch.object(a, 'api', side_effect=lambda path: missing_rows[path]), patch.object(a, 'download_verified') as download_mock, patch('sys.argv', argv):
                with self.assertRaises(KeyError):
                    a.main()
            download_mock.assert_not_called()
            self.assertFalse(missing_readback_output.exists())

    def test_output_modes_are_mutually_exclusive(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            with patch.object(a, 'api') as api_mock, patch('sys.argv', ['acquire', '--run-id', '7', '--output', str(root / 'legacy'), '--acquisition-output', str(root / 'retained')]):
                with self.assertRaises(SystemExit):
                    a.main()
            api_mock.assert_not_called()


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

    def test_download_stall_and_slow_drip_obey_wall_clock_deadline(self):
        import sys
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as t:
            root = Path(t); old = a.DOWNLOAD_TIMEOUT_SECONDS; a.DOWNLOAD_TIMEOUT_SECONDS = 0.2
            try:
                stalled = root / 'stalled.zip'
                with self.assertRaisesRegex(TimeoutError, 'timed out'):
                    a.download_verified([sys.executable, '-c', 'import time;time.sleep(10)'], stalled, 100)
                self.assertFalse(stalled.exists())
                drip = root / 'drip.zip'
                script = 'import sys,time\nfor _ in range(20):\n sys.stdout.buffer.write(b"x");sys.stdout.buffer.flush();time.sleep(.05)'
                with self.assertRaisesRegex(TimeoutError, 'timed out'):
                    a.download_verified([sys.executable, '-c', script], drip, 100)
                self.assertFalse(drip.exists())
            finally:
                a.DOWNLOAD_TIMEOUT_SECONDS = old

    def test_timeout_kills_sigterm_ignoring_producer_and_reaps_it(self):
        import signal
        import sys
        from unittest.mock import patch
        real_popen = a.subprocess.Popen
        children = []
        def start(*args, **kwargs):
            child = real_popen(*args, **kwargs); children.append(child); return child
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'sigterm.zip'; old = a.DOWNLOAD_TIMEOUT_SECONDS; a.DOWNLOAD_TIMEOUT_SECONDS = 0.15
            try:
                code = 'import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(10)'
                with patch.object(a.subprocess, 'Popen', side_effect=start):
                    with self.assertRaisesRegex(TimeoutError, 'timed out'):
                        a.download_verified([sys.executable, '-c', code], path, 100)
                self.assertEqual(len(children), 1)
                self.assertIsNotNone(children[0].poll())
                self.assertFalse(path.exists())
            finally:
                a.DOWNLOAD_TIMEOUT_SECONDS = old

    def test_interrupt_during_stream_removes_partial_file_and_reaps(self):
        import sys
        from unittest.mock import patch
        real_popen = a.subprocess.Popen; children = []
        def start(*args, **kwargs):
            child = real_popen(*args, **kwargs); children.append(child); return child
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'interrupted.zip'
            with patch.object(a.subprocess, 'Popen', side_effect=start), patch.object(a.selectors.DefaultSelector, 'select', side_effect=KeyboardInterrupt):
                with self.assertRaises(KeyboardInterrupt):
                    a.download_verified([sys.executable, '-c', 'import sys,time;sys.stdout.buffer.write(b"partial");sys.stdout.buffer.flush();time.sleep(10)'], path, 100)
            self.assertFalse(path.exists())
            self.assertEqual(len(children), 1)
            self.assertIsNotNone(children[0].poll())

    def test_gh_api_output_and_runtime_are_bounded(self):
        import sys
        from unittest.mock import patch
        with patch.object(a, 'API_COMMAND_PREFIX', [sys.executable, '-c', 'import json;print(json.dumps({"ok":True}))']):
            self.assertEqual(a.api('fixture'), {'ok': True})
        old_timeout, old_limit = a.API_TIMEOUT_SECONDS, a.API_OUTPUT_LIMIT
        try:
            a.API_TIMEOUT_SECONDS = 0.15
            with patch.object(a, 'API_COMMAND_PREFIX', [sys.executable, '-c', 'import time;time.sleep(10)']):
                with self.assertRaisesRegex(TimeoutError, 'timed out'):
                    a.api('stall')
            a.API_TIMEOUT_SECONDS = old_timeout
            a.API_OUTPUT_LIMIT = 32
            with patch.object(a, 'API_COMMAND_PREFIX', [sys.executable, '-c', 'print("x"*1000)']):
                for _ in range(8):
                    with self.subTest(over_limit_attempt=_):
                        with self.assertRaisesRegex(ValueError, 'byte limit'):
                            a.api('large')
        finally:
            a.API_TIMEOUT_SECONDS, a.API_OUTPUT_LIMIT = old_timeout, old_limit

    def test_cleanup_reaps_zombie_before_suppressing_stale_group_eperm(self):
        import signal
        from unittest.mock import patch

        class ExitedProcess:
            pid = 4242
            def __init__(self):
                self.poll_count = 0
            def poll(self):
                self.poll_count += 1
                return None if self.poll_count == 1 else 17

        process = ExitedProcess()
        with patch.object(a, '_process_group_exists', side_effect=[True, False, False, False, False]) as group_exists, \
             patch.object(a.os, 'killpg', side_effect=PermissionError('stale zombie group')) as killpg:
            a._stop_process_group(process)
        killpg.assert_called_once_with(process.pid, signal.SIGTERM)
        self.assertGreaterEqual(process.poll_count, 4)
        self.assertEqual(group_exists.call_count, 5)

    def test_cleanup_keeps_live_group_eperm_fail_closed(self):
        import signal
        from unittest.mock import patch

        class LiveProcess:
            pid = 4343
            def poll(self):
                return None

        process = LiveProcess()
        with patch.object(a, '_process_group_exists', return_value=True) as group_exists, \
             patch.object(a.os, 'killpg', side_effect=PermissionError('inaccessible live group')) as killpg:
            with self.assertRaisesRegex(PermissionError, 'inaccessible live group'):
                a._stop_process_group(process)
        group_exists.assert_called_once_with(process.pid)
        killpg.assert_called_once_with(process.pid, signal.SIGTERM)

    def test_parent_exit_with_child_holding_stdout_obeys_deadline(self):
        import sys
        import time
        old = a.DOWNLOAD_TIMEOUT_SECONDS; a.DOWNLOAD_TIMEOUT_SECONDS = 0.2
        with tempfile.TemporaryDirectory() as t:
            path = Path(t) / 'orphaned-pipe.zip'
            script = 'import os,sys,time;pid=os.fork();(time.sleep(10),sys.stdout.buffer.write(b"late")) if pid==0 else os._exit(0)'
            try:
                started = time.monotonic()
                with self.assertRaisesRegex(TimeoutError, 'timed out'):
                    a.download_verified([sys.executable, '-c', script], path, 100)
                self.assertLess(time.monotonic() - started, 3)
                self.assertFalse(path.exists())
            finally:
                a.DOWNLOAD_TIMEOUT_SECONDS = old

    def test_subprocess_timeout_rejects_nonfinite_and_excessive_values(self):
        for timeout in (float('nan'), float('inf'), -float('inf'), a.MAX_SUBPROCESS_TIMEOUT_SECONDS + 1):
            with self.subTest(timeout=timeout), self.assertRaisesRegex(ValueError, 'time limit'):
                a._run_bounded_process(['unused'], 10, timeout)


    def test_artifact_origin_must_match_run_repository_and_head(self):
        for key, value in [('id', 8), ('head_sha', '2' * 40), ('repository_id', 999), ('head_repository_id', 999)]:
            item = copy.deepcopy(self.artifact)
            item['workflow_run'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                a.select_artifact(self.run, {'total_count': 1, 'artifacts': [item]}, 7, self.sha)
        item = dict(self.artifact)
        del item['workflow_run']
        with self.assertRaises(ValueError):
            a.select_artifact(self.run, {'total_count': 1, 'artifacts': [item]}, 7, self.sha)

    def test_main_dispatch_run_requires_exact_successful_same_repo_workflow(self):
        run = dict(self.run, event='workflow_dispatch', head_branch='main', pull_requests=[])
        self.assertEqual(a.validate_main_dispatch_run(run, 7), self.sha)
        bad_values = [
            ('id', 8), ('event', 'pull_request'), ('event', 'push'),
            ('head_branch', 'feature'), ('head_branch', 'mainline'),
            ('path', 'other.yml'), ('status', 'in_progress'),
            ('conclusion', 'failure'), ('head_sha', 'A' * 40),
            ('head_sha', '2' * 39),
            ('repository', {'full_name': a.REPOSITORY, 'id': 124}),
            ('head_repository', {'full_name': 'fork/kairos', 'id': 123}),
        ]
        for key, value in bad_values:
            changed = copy.deepcopy(run)
            changed[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                a.validate_main_dispatch_run(changed, 7)
        with self.assertRaises(ValueError):
            a.validate_main_dispatch_run(dict(run, pull_requests=[{'number': 1}]), 7)
        with self.assertRaises(ValueError):
            a.validate_main_dispatch_run(dict(run, head_repository={'full_name': a.REPOSITORY, 'id': 124}), 7)
        with self.assertRaises(ValueError):
            a.validate_main_dispatch_run(dict(run, repository={'full_name': 'elsewhere/repo', 'id': 123}), 7)

    def test_main_branch_and_compare_contract_uses_real_api_shape(self):
        source = 'a' * 40
        observed_main = 'b' * 40
        branch = {'name': 'main', 'commit': {'sha': observed_main}}
        url = f'https://api.github.com/repos/{a.REPOSITORY}/compare/{source}...{observed_main}'
        compare = {
            'url': url,
            'base_commit': {'sha': source},
            'merge_base_commit': {'sha': source},
            'status': 'ahead',
            'ahead_by': 3,
            'behind_by': 0,
        }
        result = a.validate_main_ancestry(source, branch, compare)
        self.assertEqual(result['observed_main_sha'], observed_main)
        self.assertEqual(result['compare_url'], url)
        self.assertEqual(result['merge_base_sha'], source)
        self.assertEqual(result['ahead_by'], 3)
        self.assertNotIn('head_commit', compare)

        identity = a.validate_main_ancestry(source, {'name': 'main', 'commit': {'sha': source}}, None)
        self.assertEqual(identity['status'], 'identical')
        self.assertIsNone(identity['compare_url'])
        for bad_branch in [
            {'name': 'master', 'commit': {'sha': observed_main}},
            {'name': 'main', 'commit': {'sha': 'B' * 40}},
            {'name': 'main', 'commit': {}},
        ]:
            with self.subTest(branch=bad_branch), self.assertRaises(ValueError):
                a.validate_main_ancestry(source, bad_branch, compare)
        mutations = [
            ('url', 'https://api.github.com/repos/elsewhere/repo/compare/' + source + '...' + observed_main),
            ('base_commit', {'sha': observed_main}),
            ('merge_base_commit', {'sha': observed_main}),
            ('status', 'behind'), ('ahead_by', True), ('ahead_by', 0), ('ahead_by', -1),
            ('behind_by', True), ('behind_by', 1),
        ]
        for key, value in mutations:
            changed = copy.deepcopy(compare)
            changed[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                a.validate_main_ancestry(source, branch, changed)

    def test_strict_main_artifact_requires_main_origin_metadata(self):
        run = dict(self.run, event='workflow_dispatch', head_branch='main', pull_requests=[])
        item = copy.deepcopy(self.artifact)
        item['workflow_run']['head_branch'] = 'main'
        inventory = {'total_count': 1, 'artifacts': [item]}
        self.assertEqual(a.select_artifact(run, inventory, 7, self.sha, require_main_dispatch=True), item)
        for key, value in [('id', 8), ('repository_id', 999), ('head_repository_id', 999), ('head_sha', '2' * 40), ('head_branch', 'feature')]:
            bad = copy.deepcopy(item)
            bad['workflow_run'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                a.select_artifact(run, {'total_count': 1, 'artifacts': [bad]}, 7, self.sha, require_main_dispatch=True)

    def test_strict_cli_derives_source_sha_and_rejects_before_inventory(self):
        import io
        import json
        import tarfile
        from unittest.mock import patch
        bundle_spec = importlib.util.spec_from_file_location('strict_bundle_test', Path(a.__file__).with_name('build_package_archive_bundle.py'))
        bundle = importlib.util.module_from_spec(bundle_spec)
        bundle_spec.loader.exec_module(bundle)
        source = self.sha
        observed_main = '2' * 40
        run = dict(self.run, event='workflow_dispatch', head_branch='main', pull_requests=[])
        branch = {'name': 'main', 'commit': {'sha': observed_main}}
        compare_url = f'https://api.github.com/repos/{a.REPOSITORY}/compare/{source}...{observed_main}'
        compare = {'url': compare_url, 'base_commit': {'sha': source}, 'merge_base_commit': {'sha': source}, 'status': 'ahead', 'ahead_by': 1, 'behind_by': 0}
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            package_source = root / 'package-source'
            for ecosystem, (_, extensions) in bundle.ARCHIVES.items():
                directory = package_source / ecosystem
                directory.mkdir(parents=True)
                (directory / 'BUILD-INFO.json').write_text(json.dumps({'ecosystem': ecosystem, 'source_commit': source, 'command': 'fixture', 'toolchain': 'fixture', 'platform': 'fixture', 'exit_status': 0}))
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
            bundle.build(package_source, retained, source)
            archive_zip = root / 'download.zip'
            with zipfile.ZipFile(archive_zip, 'w') as z:
                for item in retained.rglob('*'):
                    if item.is_file():
                        z.write(item, item.relative_to(retained).as_posix())
            payload = archive_zip.read_bytes()
            digest = 'sha256:' + hashlib.sha256(payload).hexdigest()
            self.artifact['digest'] = digest
            self.artifact['workflow_run']['head_branch'] = 'main'
            inventory = {'total_count': 1, 'artifacts': [self.artifact]}
            api_rows = {
                f'repos/{a.REPOSITORY}/actions/runs/7': run,
                f'repos/{a.REPOSITORY}/branches/main': branch,
                f'repos/{a.REPOSITORY}/compare/{source}...{observed_main}': compare,
                f'repos/{a.REPOSITORY}/actions/runs/7/artifacts?per_page=100': inventory,
            }
            calls = []
            def fake_api(path):
                calls.append(path)
                return api_rows[path]
            def fake_download(argv, path):
                self.assertEqual(argv[-1], f'repos/{a.REPOSITORY}/actions/artifacts/9/zip')
                path.write_bytes(payload)
                return digest
            output = root / 'archives'
            argv = ['acquire', '--run-id', '7', '--output', str(output), '--require-main-dispatch']
            with patch.object(a, 'api', side_effect=fake_api), patch.object(a, 'download_verified', side_effect=fake_download), patch('sys.argv', argv):
                a.main()
            self.assertEqual(calls, list(api_rows))
            bundle.verify(output)
            receipt = json.loads(output.with_name('archives.acquisition.json').read_text())
            self.assertEqual(receipt['source_commit'], source)
            self.assertEqual(receipt['head_commit'], source)
            self.assertEqual(receipt['artifact_digest'], digest)
            self.assertEqual(receipt['main_ancestry']['observed_main_sha'], observed_main)
            self.assertEqual(receipt['main_ancestry']['merge_base_sha'], source)
            self.assertEqual(receipt['selection_policy'], 'same-repository-main-workflow-dispatch')

            identity_branch = {'name': 'main', 'commit': {'sha': source}}
            identity_inventory = {'total_count': 1, 'artifacts': [self.artifact]}
            identity_rows = {
                f'repos/{a.REPOSITORY}/actions/runs/7': run,
                f'repos/{a.REPOSITORY}/branches/main': identity_branch,
                f'repos/{a.REPOSITORY}/actions/runs/7/artifacts?per_page=100': identity_inventory,
            }
            identity_calls = []
            def identity_api(path):
                identity_calls.append(path)
                return identity_rows[path]
            identity_output = root / 'identity-archives'
            identity_argv = ['acquire', '--run-id', '7', '--output', str(identity_output), '--require-main-dispatch']
            with patch.object(a, 'api', side_effect=identity_api), patch.object(a, 'download_verified', side_effect=fake_download), patch('sys.argv', identity_argv):
                a.main()
            self.assertEqual(identity_calls, list(identity_rows))
            identity_receipt = json.loads(identity_output.with_name('identity-archives.acquisition.json').read_text())
            self.assertEqual(identity_receipt['main_ancestry']['status'], 'identical')
            self.assertIsNone(identity_receipt['main_ancestry']['compare_url'])

            malformed_branches = [
                {'name': 'master', 'commit': {'sha': observed_main}},
                {'name': 'main', 'commit': None},
                {'name': 'main', 'commit': []},
                {'name': 'main', 'commit': {'sha': '2' * 39}},
            ]
            for index, malformed_branch in enumerate(malformed_branches):
                malformed_output = root / f'malformed-branch-{index}'
                malformed_argv = ['acquire', '--run-id', '7', '--output', str(malformed_output), '--require-main-dispatch']
                with self.subTest(branch=malformed_branch), patch.object(a, 'api', side_effect=[run, malformed_branch]) as api_mock, patch.object(a, 'download_verified') as download_mock, patch('sys.argv', malformed_argv):
                    with self.assertRaises(ValueError):
                        a.main()
                self.assertEqual(api_mock.call_count, 2)
                download_mock.assert_not_called()

            invalid_run = dict(run, event='pull_request')
            invalid_argv = ['acquire', '--run-id', '7', '--output', str(root / 'invalid-run'), '--require-main-dispatch']
            with patch.object(a, 'api', side_effect=[invalid_run]) as api_mock, patch.object(a, 'download_verified') as download_mock, patch('sys.argv', invalid_argv):
                with self.assertRaises(ValueError):
                    a.main()
            self.assertEqual(api_mock.call_count, 1)
            download_mock.assert_not_called()

            bad_compare = dict(compare, behind_by=1)
            bad_compare_argv = ['acquire', '--run-id', '7', '--output', str(root / 'bad-compare'), '--require-main-dispatch']
            with patch.object(a, 'api', side_effect=[run, branch, bad_compare]) as api_mock, patch.object(a, 'download_verified') as download_mock, patch('sys.argv', bad_compare_argv):
                with self.assertRaises(ValueError):
                    a.main()
            self.assertEqual(api_mock.call_count, 3)
            download_mock.assert_not_called()

            with patch.object(a, 'api') as api_mock, patch('sys.argv', ['acquire', '--run-id', '7', '--source-commit', source, '--head-commit', source, '--output', str(root / 'strict-override'), '--require-main-dispatch']):
                with self.assertRaises(SystemExit):
                    a.main()
            api_mock.assert_not_called()

if __name__ == '__main__':
    unittest.main()
