"""Exercise actual local environments and fail-closed runtime handover."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/python_runtime.py'


class LocalPythonTests(unittest.TestCase):
    def invoke(self, root, *extra, env=None):
        return subprocess.run([sys.executable, str(SCRIPT), '--root', str(root), *extra],
                              text=True, capture_output=True, env=env)

    def test_real_environment_child_and_reuse_preserve_existing_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            result = self.invoke(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(result.stdout)
            self.assertEqual(receipt['version'], '3.15.1')
            self.assertEqual(Path(receipt['prefix']), root / '.venv')
            executable = receipt['executable']
            if os.name != 'nt':
                self.assertTrue(Path(executable).is_symlink())
            child = subprocess.check_output([executable, '-c', 'import sys; print(sys.version_info[:3]); print(sys.prefix)'], text=True)
            self.assertIn('(3, 15, 1)', child)
            self.assertIn(str(root / '.venv'), child)
            marker = root / '.venv' / 'preserve-me'
            marker.write_text('in-flight recovery state')
            config = (root / '.venv/pyvenv.cfg').read_bytes()
            path_output = root / 'github-path'
            env_output = root / 'github-env'
            second = self.invoke(root, '--github', env=dict(os.environ, GITHUB_PATH=str(path_output), GITHUB_ENV=str(env_output)))
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertEqual(path_output.read_text().strip(), str(Path(executable).parent))
            self.assertIn('KAIROS_PYTHON=' + executable, env_output.read_text())
            child_env = dict(os.environ, PATH=str(Path(executable).parent) + os.pathsep + os.environ['PATH'])
            actual = subprocess.check_output(['python', '-c', 'import sys; print(sys.prefix)'], env=child_env, text=True).strip()
            self.assertEqual(Path(actual), root / '.venv')
            self.assertEqual(marker.read_text(), 'in-flight recovery state')
            self.assertEqual((root / '.venv/pyvenv.cfg').read_bytes(), config)

    def test_existing_unmanaged_environment_is_not_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / '.venv'
            target.mkdir()
            marker = target / 'preserve-me'
            marker.write_text('private state')
            result = self.invoke(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(marker.read_text(), 'private state')
            self.assertFalse((target / 'pyvenv.cfg').exists())

    def test_base_interpreter_alias_cannot_pass_as_a_local_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / '.venv/bin/python'
            executable.parent.mkdir(parents=True)
            executable.symlink_to(sys._base_executable)
            result = self.invoke(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('prefix', result.stderr)
            self.assertTrue(executable.is_symlink())

    def test_symlink_environment_rejected_without_mutating_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            external = root / 'external'
            external.mkdir()
            (root / '.venv').symlink_to(external, target_is_directory=True)
            result = self.invoke(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(list(external.iterdir()), [])

    def test_wrong_bootstrap_rejects_before_environment_creation(self):
        import importlib.util
        from unittest.mock import patch
        spec = importlib.util.spec_from_file_location('python_runtime', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(module.sys, 'version_info', (3, 16, 0, 'alpha', 1)):
                with self.assertRaisesRegex(RuntimeError, '3.15.1'):
                    module.prepare(root)
            self.assertFalse((root / '.venv').exists())

    def test_failed_verification_does_not_export_github_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.venv').mkdir()
            output = root / 'path-output'
            env = dict(os.environ, GITHUB_PATH=str(output), GITHUB_ENV=str(root / 'env-output'))
            result = self.invoke(root, '--github', env=env)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(output.exists())

    def test_python_jobs_bootstrap_after_source_and_use_verified_runtime(self):
        import re
        for workflow in sorted((ROOT / '.github/workflows').glob('*.yml')):
            for job in re.split(r'(?m)(?=^  [A-Za-z0-9_-]+:\n)', workflow.read_text()):
                if 'actions/setup-python@' not in job:
                    continue
                with self.subTest(workflow=workflow.name, job=job.splitlines()[0]):
                    self.assertIn('python-version:', job)
                    self.assertIn('3.15.1', job)
                    if workflow.name == 'archive-supply-chain-main.yml' and job.startswith('  acquire:'):
                        self.assertLess(job.index('actions/setup-python@'), job.index('id: dispatch-pins'))
                        self.assertIn('Bootstrap isolated Python for dispatch pin validation', job)
                    else:
                        self.assertLess(job.index('actions/checkout@'), job.index('actions/setup-python@'))
                    guard = 'run: python scripts/python_runtime.py --github'
                    self.assertLess(job.index('actions/setup-python@'), job.index(guard))
                    self.assertIn('scripts/python_runtime.py --github', job)

    def test_dispatch_preflight_uses_real_local_runtime_without_source_or_api(self):
        import shlex
        import textwrap
        workflow = (ROOT / '.github/workflows/archive-supply-chain-main.yml').read_text()
        step = workflow.split('      - name: Bootstrap isolated Python for dispatch pin validation\n', 1)[1].split('\n      - ', 1)[0]
        script = textwrap.dedent(step.split('        run: |\n', 1)[1])
        script = script.replace('python -I -', shlex.quote(sys.executable) + ' -I -', 1)
        self.assertNotIn('gh api', script)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / 'github-path'
            result = subprocess.run(['bash', '-euc', script], text=True, capture_output=True,
                                    env=dict(os.environ, GITHUB_WORKSPACE=str(root), GITHUB_PATH=str(output)))
            self.assertEqual(result.returncode, 0, result.stderr)
            version, prefix = json.loads(result.stdout)
            self.assertEqual(version, [3, 15, 1])
            self.assertEqual(Path(prefix), root / '.venv')
            self.assertEqual(output.read_text().strip(), str(root / '.venv/bin'))


if __name__ == '__main__':
    unittest.main()
