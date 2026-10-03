"""PR195's human-approved narrow exception; independent scopes fail closed."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions'
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module
runner = load('runner195', ROOT / 'scripts/bootstrap-node-tools/run_npm_audit_gate.py')
policy_module = load('policy195', BASE / 'npm_audit_policy.py')

class Scoped195Tests(unittest.TestCase):
    def setUp(self):
        self.policy = json.loads((BASE / 'EXC-195-http-cache.json').read_text())
        self.report = json.loads((BASE / 'EXC-193-http-cache-audit-baseline.json').read_text())
        self.now = datetime(2026, 10, 5, tzinfo=timezone.utc)

    def classify(self, policy=None, context='development_pr_195', number=195):
        return policy_module.classify(self.report, 1, policy or self.policy, context, number, self.now)

    def test_approved_195_graph(self):
        self.assertEqual(self.classify(), 'approved_temporary_exception')

    def test_number_context_identity_and_expiry_rejected(self):
        for number in [193,196,True]:
            with self.subTest(number=number), self.assertRaises(ValueError):
                self.classify(number=number)
        for context in ['development_pr_193','alpha_package_dry_run','publication','release_candidate']:
            with self.subTest(context=context), self.assertRaises(ValueError):
                self.classify(context=context)
        for field,value in [('id','EXC-999'),('required_pull_request',193),('status','proposed'),('expires_at','2026-10-04T00:00:00+10:00')]:
            policy=copy.deepcopy(self.policy);policy[field]=value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.classify(policy=policy)

    def test_event_scope(self):
        def event(number=195,branch='codex/vitest-floor-closeout-20261003',repo='edithatogo/kairos'):
            return {'repository':{'full_name':'edithatogo/kairos'},'pull_request':{'number':number,'head':{'ref':branch,'repo':{'full_name':repo}}}}
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'event.json'
            env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'pull_request','GITHUB_EVENT_PATH':str(path),'GITHUB_REPOSITORY':'edithatogo/kairos'}
            with patch.dict(os.environ,env,clear=True):
                path.write_text(json.dumps(event()))
                self.assertEqual(runner.selected_policy_name(),'EXC-195-http-cache.json')
                self.assertEqual(runner.execution_context(self.policy),('development_pr_195',195))
                for bad in [event(branch='codex/other'),event(repo='fork/kairos')]:
                    path.write_text(json.dumps(bad))
                    with self.assertRaises(ValueError):runner.execution_context(self.policy)
                path.write_text(json.dumps(event(number=196)))
                self.assertEqual(runner.selected_policy_name(),'EXC-193-http-cache.json')
                with self.assertRaises(ValueError):self.classify(number=196)

if __name__=='__main__':unittest.main()
