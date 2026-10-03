"""Real Git/worktree/process tests for advisory session coordination."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[2] / 'scripts/agent_session.py'
spec = importlib.util.spec_from_file_location('agent_session', SCRIPT)
session = importlib.util.module_from_spec(spec)
spec.loader.exec_module(session)


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'repo'
        self.root.mkdir()
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.invalid')
        self.git('config', 'user.name', 'Test')
        (self.root / 'contract.md').write_text('bounded contract\n')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True, stderr=subprocess.DEVNULL).strip()

    def worktree(self):
        path = Path(self.temp.name) / 'worker'
        self.git('worktree', 'add', '-q', '-b', 'worker', str(path))
        return path

    def test_two_real_processes_racing_have_one_winner(self):
        commands = [[sys_executable(), str(SCRIPT), '--root', str(self.root), 'claim', '--owner', owner, '--task', 'fixture', '--paths', 'src'] for owner in ('a', 'b')]
        processes = [subprocess.Popen(c, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) for c in commands]
        results = [p.communicate(timeout=10) for p in processes]
        self.assertEqual(sorted(p.returncode for p in processes), [0, 1])
        winner = json.loads(results[next(i for i, p in enumerate(processes) if p.returncode == 0)][0])
        self.assertEqual(session.check(self.root, winner['token'])['task'], 'fixture')

    def test_same_checkout_disjoint_paths_still_conflict(self):
        session.claim(self.root, 'a', 'first', ['src'])
        with self.assertRaisesRegex(ValueError, 'writer conflict'):
            session.claim(self.root, 'b', 'second', ['docs'])

    def test_separate_worktrees_share_path_reservations(self):
        other = self.worktree()
        session.claim(self.root, 'a', 'first', ['src'])
        with self.assertRaisesRegex(ValueError, 'writer conflict'):
            session.claim(other, 'b', 'second', ['src/nested'])
        lease = session.claim(other, 'b', 'second', ['docs'])
        self.assertEqual(session.check(other, lease['token'])['owner'], 'b')
        with self.assertRaisesRegex(ValueError, 'wrong worktree'):
            session.check(self.root, lease['token'])

    def test_expiry_never_automatically_steals_claim(self):
        lease = session.claim(self.root, 'a', 'first', ['src'])
        with session.store(self.root) as (_, _, state):
            state['leases'][0]['expires_at'] = time.time() - 1
        with self.assertRaises(ValueError):
            session.heartbeat(self.root, lease['token'])
        with self.assertRaises(ValueError):
            session.claim(self.root, 'b', 'second', ['docs'])
        with self.assertRaises(ValueError):
            session.recover(self.root, lease['token'], 'time passed')
        session.recover(self.root, lease['token'], 'verified worker stopped', True)
        session.claim(self.root, 'b', 'second', ['docs'])
        with session.store(self.root) as (_, _, state):
            self.assertEqual(state['events'][-2]['event'], 'recover')

    def test_heartbeat_release_and_head_drift(self):
        lease = session.claim(self.root, 'a', 'first', ['src'])
        self.assertGreater(session.heartbeat(self.root, lease['token'], 1000)['expires_at'], lease['expires_at'])
        with self.assertRaises(ValueError):
            session.check(self.root, lease['token'], ['docs'])
        self.git('commit', '--allow-empty', '-qm', 'drift')
        with self.assertRaisesRegex(ValueError, 'HEAD drift'):
            session.check(self.root, lease['token'])
        session.release(self.root, lease['token'])
        session.claim(self.root, 'b', 'second', ['src'])

    def test_path_traversal_symlink_and_unknown_token_rejected(self):
        (self.root / 'alias').symlink_to('contract.md')
        for path in ('../escape', '/absolute', '.git/config', 'alias'):
            with self.assertRaises(ValueError):
                session.claim(self.root, 'a', 'first', [path])
        with self.assertRaises(ValueError):
            session.check(self.root, 'unknown')

    def test_context_has_exact_hash_and_rejects_untracked_or_oversize(self):
        result = session.snapshot(self.root, 'fixture', ['contract.md'])
        self.assertEqual(result['head_sha'], self.git('rev-parse', 'HEAD'))
        first_hash = result['documents'][0]['sha256']
        (self.root / 'contract.md').write_text('changed\n')
        self.assertNotEqual(session.snapshot(self.root, 'fixture', ['contract.md'])['documents'][0]['sha256'], first_hash)
        self.assertTrue(session.snapshot(self.root, 'fixture', ['contract.md'])['git_status'])
        (self.root / 'private.md').write_text('untracked')
        with self.assertRaises(ValueError):
            session.snapshot(self.root, 'fixture', ['private.md'])
        with self.assertRaisesRegex(ValueError, 'byte budget'):
            session.snapshot(self.root, 'fixture', ['contract.md'], 1)

    def test_corrupted_store_fails_closed(self):
        _, folder = session.identity(self.root)
        folder.mkdir()
        (folder / 'state.json').write_text('{broken')
        with self.assertRaises(ValueError):
            session.claim(self.root, 'a', 'fixture', ['src'])

    def test_bound_input_changes_fail_before_a_write_check(self):
        lease = session.claim(self.root, 'a', 'fixture', ['src'], inputs=['contract.md'])
        session.check(self.root, lease['token'], ['src/new.rs'])
        (self.root / 'contract.md').write_text('changed contract\n')
        with self.assertRaisesRegex(ValueError, 'input hash drift'):
            session.check(self.root, lease['token'])

    def test_dirty_claim_rejects_staged_unstaged_and_untracked(self):
        (self.root / 'contract.md').write_text('unstaged')
        with self.assertRaisesRegex(ValueError, 'clean'):
            session.claim(self.root, 'a', 'fixture', ['contract.md'])
        self.git('add', 'contract.md')
        with self.assertRaisesRegex(ValueError, 'clean'):
            session.claim(self.root, 'a', 'fixture', ['contract.md'])
        self.git('reset', '--hard', '-q', 'HEAD')
        (self.root / 'unknown.txt').write_text('untracked')
        with self.assertRaisesRegex(ValueError, 'clean'):
            session.claim(self.root, 'a', 'fixture', ['unknown.txt'])

    def test_actual_out_of_scope_changes_cannot_be_omitted(self):
        lease = session.claim(self.root, 'a', 'fixture', ['src'])
        for operation in ('edit', 'delete', 'create', 'rename'):
            with self.subTest(operation=operation):
                if operation == 'edit': (self.root / 'contract.md').write_text('edit')
                elif operation == 'delete': (self.root / 'contract.md').unlink()
                elif operation == 'create': (self.root / 'other.txt').write_text('new')
                else: self.git('mv', 'contract.md', 'renamed.md')
                with self.assertRaisesRegex(ValueError, 'outside reservation'):
                    session.check(self.root, lease['token'])
                self.git('reset', '--hard', '-q', 'HEAD')
                if (self.root / 'other.txt').exists(): (self.root / 'other.txt').unlink()

    def test_ignored_build_artifacts_are_not_source_authority(self):
        (self.root / '.gitignore').write_text('build/\n')
        self.git('add', '.gitignore'); self.git('commit', '-qm', 'ignore')
        (self.root / 'build').mkdir(); (self.root / 'build/output').write_text('build')
        lease = session.claim(self.root, 'a', 'fixture', ['src'])
        session.check(self.root, lease['token'])
        with self.assertRaisesRegex(ValueError, 'outside reservation'):
            session.check(self.root, lease['token'], ['build/output'])

    def test_invalid_utf8_context_is_sanitized(self):
        (self.root / 'binary').write_bytes(b'\xff')
        self.git('add', 'binary'); self.git('commit', '-qm', 'binary')
        with self.assertRaisesRegex(ValueError, 'UTF-8'):
            session.snapshot(self.root, 'fixture', ['binary'])

    def test_case_and_unicode_aliases_conflict_conservatively(self):
        other = self.worktree()
        session.claim(self.root, 'a', 'fixture', ['Docs', 'caf\u00e9'])
        for path in ('docs/plan.md', 'cafe\u0301/input'):
            with self.assertRaisesRegex(ValueError, 'writer conflict'):
                session.claim(other, 'b', 'fixture', [path])

    def test_malformed_json_shapes_fail_closed_without_traceback(self):
        valid = session.claim(self.root, 'a', 'fixture', ['src'])
        for malformed in ([], {'schema_version': 1, 'leases': [{}], 'events': []},
                          {'schema_version': 1, 'leases': [], 'events': [{}]},
                          {'schema_version': 1, 'leases': [dict(valid, expires_at=float('nan'))], 'events': []},
                          {'schema_version': 1, 'leases': [dict(valid, expires_at=float('inf'))], 'events': []},
                          {'schema_version': 1, 'leases': [dict(valid, expires_at='wrong')], 'events': []}):
            with self.subTest(state=malformed):
                _, folder = session.identity(self.root)
                (folder / 'state.json').write_text(json.dumps(malformed))
                run = subprocess.run([sys_executable(), str(SCRIPT), '--root', str(self.root), 'status'], capture_output=True, text=True)
                self.assertEqual(run.returncode, 1)
                self.assertIn('FAIL:', run.stderr)
                self.assertNotIn('Traceback', run.stderr)

    def test_case_alias_cannot_expand_exact_write_scope(self):
        lease = session.claim(self.root, 'a', 'fixture', ['Docs'])
        (self.root / 'docs').mkdir(); (self.root / 'docs/plan.md').write_text('out of exact scope')
        with self.assertRaisesRegex(ValueError, 'outside reservation'):
            session.check(self.root, lease['token'])

    def test_oversized_context_rejected_before_file_allocation(self):
        (self.root / 'large.md').write_text('x' * 100000)
        self.git('add', 'large.md'); self.git('commit', '-qm', 'large')
        with patch.object(Path, 'open', side_effect=AssertionError('must not read oversized file')):
            with self.assertRaisesRegex(ValueError, 'before read'):
                session.snapshot(self.root, 'fixture', ['large.md'], 100)


def sys_executable():
    import sys
    return sys.executable


if __name__ == '__main__':
    unittest.main()
