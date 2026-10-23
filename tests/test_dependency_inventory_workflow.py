"""Execute the actual native-boundary scripts with fictional, network-free clients."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / '.github/workflows/python-dependency-inventory.yml'
MANIFESTS = {'bootstrap': 'scripts/bootstrap-python-tools.lock', 'archive': 'scripts/archive-python-tools.lock',
             'package': 'scripts/package-python-tools.lock', 'gym': 'python/kairo_gym/ci-requirements.lock'}
SHA = 'a' * 40


def script(name, key='script'):
    step = WORKFLOW.read_text().split('      - name: ' + name + '\n', 1)[1].split('\n      - ', 1)[0]
    return textwrap.dedent(step.split(key + ': |\n', 1)[1])


def fixture(root):
    output = root / 'artifacts/python-dependency-inventory'
    sources = {}
    for identifier, manifest in MANIFESTS.items():
        leaf = output / identifier
        leaf.mkdir(parents=True)
        sources[manifest] = 'fictional-' + identifier + '==1.2.3\n'
        payload = {'version': 0, 'sha': SHA, 'ref': 'refs/heads/main',
                   'job': {'id': '1.' + identifier, 'correlator': 'kairos-python315-inventory:' + identifier},
                   'detector': {'name': 'careops-pip-inspect-inventory'}, 'scanned': '2026-10-24T00:00:00Z',
                   'metadata': {'input_contract': 'installed-tooling-inventory-no-edge-or-extras-claims'},
                   'manifests': {manifest: {'resolved': {'pkg:pypi/fictional-' + identifier + '@1.2.3': {'package_url': 'pkg:pypi/fictional-' + identifier + '@1.2.3'}}}}}
        (leaf / 'snapshot.json').write_text(json.dumps(payload))
        receipt = {'sourceCommit': SHA, 'manifest': manifest, 'python': '3.15.1', 'localPrefixVerified': True,
                   'lockSha256': hashlib.sha256(sources[manifest].encode()).hexdigest()}
        (leaf / 'receipt.json').write_text(json.dumps(receipt))
    (root / 'sources.json').write_text(json.dumps(sources))
    return output


def execute(root, body, mode):
    (root / 'body.txt').write_text(body)
    wrapper = r'''
const fs = require('node:fs');
const sources = JSON.parse(fs.readFileSync('sources.json'));
const mode = process.argv[2];
let posts = 0, gets = 0, fetches = 0;
const context = {sha:'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',ref:mode==='wrong_context'?'refs/pull/1/merge':'refs/heads/main',eventName:'push',runId:1,repo:{owner:'fictional',repo:'fixture'}};
const github = {rest:{repos:{getContent:async ({path}) => {gets++;return {data:{content:Buffer.from(sources[path]).toString('base64')}};}},dependencyGraph:{createRepositorySnapshot:async payload => {posts++;if(mode==='post_failure')throw Object.assign(new Error('fictional503'),{status:503});return {status:201,data:{id:100+posts}};}}},request:async route => {
 if(mode==='foreign_report' && route.endsWith('/generate-report'))return {status:201,data:{sbom_url:'https://example.invalid/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'}};
 if(route.endsWith('/generate-report'))return {status:201,data:{sbom_url:'https://api.github.com/repos/fictional/fixture/dependency-graph/sbom/fetch-report/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'}};
 fetches++;
 if(mode==='denied')throw Object.assign(new Error('fictional403'),{status:403});
 if(mode==='not_ready' || (mode==='wait' && fetches===1))return {status:202,data:''};
 const packages=mode==='missing'?[]:Object.keys(sources).map((_,i)=>({externalRefs:[{referenceType:'purl',referenceLocator:'pkg:pypi/fictional-'+['bootstrap','archive','package','gym'][i]+'@1.2.3'}]}));
 const data={spdxVersion:'SPDX-2.3',packages};
 return {status:200,data:mode==='binary'?Buffer.from(JSON.stringify(data)):data};
}};
const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor;
(async()=>{let ok=true,error=null;try{await new AsyncFunction('require','github','context','core','setTimeout',fs.readFileSync('body.txt','utf8'))(require,github,context,{info:()=>{}},callback=>callback());}catch(e){ok=false;error=e.message;}console.log(JSON.stringify({ok,error,posts,gets,fetches}));})();
'''
    (root / 'runner.cjs').write_text(wrapper)
    result = subprocess.run(['node', 'runner.cjs', mode], cwd=root, check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


class InventoryWorkflowTests(unittest.TestCase):
    def test_only_trusted_main_can_write_and_readback_has_no_write_permission(self):
        text = WORKFLOW.read_text()
        prepare, submit = text.split('  submit:\n', 1)
        submit, verify = submit.split('  verify:\n', 1)
        self.assertNotIn('contents: write', prepare)
        self.assertIn("github.ref == 'refs/heads/main'", submit)
        self.assertIn("github.event_name == 'push' || github.event_name == 'workflow_dispatch'", submit)
        self.assertIn('contents: write', submit)
        self.assertNotIn('actions/checkout', submit)
        self.assertIn('contents: read', verify)
        self.assertNotIn('contents: write', verify)
        self.assertNotIn('createRepositorySnapshot', verify)
        self.assertIn('retries: 0', submit)
        self.assertIn('retries: 0', verify)
        self.assertIn('cb62ad8666a4c6282f0e4c3dacca5cf4226ed2c4', prepare)
        self.assertIn('--only-binary=:all: --require-hashes', prepare)
        self.assertIn('sbom/generate-report', verify)
        self.assertNotIn('exportSbom', verify)

    def test_actual_submit_validates_all_source_locks_before_any_mutation(self):
        body = script('Submit validated source-bound payloads without retries')
        for mode in ['ok', 'bad_lock', 'bad_ref', 'extra_transport', 'wrong_context']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                output = fixture(root)
                if mode == 'bad_lock':
                    path = output / 'gym/receipt.json'; value = json.loads(path.read_text()); value['lockSha256'] = '0' * 64; path.write_text(json.dumps(value))
                if mode in ['bad_ref', 'extra_transport']:
                    path = output / 'gym/snapshot.json'; value = json.loads(path.read_text())
                    if mode == 'bad_ref': value['ref'] = 'refs/heads/other'
                    else: value['request'] = {'baseUrl': 'https://example.invalid'}
                    path.write_text(json.dumps(value))
                result = execute(root, body, mode)
                self.assertEqual(result['ok'], mode == 'ok', result)
                self.assertEqual(result['posts'], 4 if mode == 'ok' else 0)
                if mode == 'ok':
                    receipt = json.loads((output / 'accepted.json').read_text())
                    self.assertEqual(receipt['state'], 'accepted')
                    self.assertEqual(len(receipt['submissions']), 4)

    def test_native_write_failure_is_retained_and_never_retried(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); output = fixture(root)
            result = execute(root, script('Submit validated source-bound payloads without retries'), 'post_failure')
            self.assertFalse(result['ok']); self.assertEqual(result['posts'], 1)
            value = json.loads((output / 'accepted.json').read_text())
            self.assertNotEqual(value['state'], 'accepted')
            self.assertEqual(value['submissions'][0]['state'], 'started')
            self.assertNotIn('snapshotId', value['submissions'][0])

    def test_readback_polls_only_documented_processing_and_preserves_missing_or_denial(self):
        for mode in ['wait', 'missing', 'denied', 'not_ready', 'foreign_report', 'binary']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary); fixture(root)
                self.assertTrue(execute(root, script('Submit validated source-bound payloads without retries'), 'ok')['ok'])
                result = execute(root, script('Verify accepted responses and actual graph presence'), mode)
                self.assertEqual(result['ok'], mode in ['wait', 'binary'], result)
                self.assertEqual(result['posts'], 0)
                self.assertEqual(result['fetches'], 2 if mode == 'wait' else 6 if mode == 'not_ready' else 0 if mode == 'foreign_report' else 1)

    def test_terminal_receipt_runs_when_python_was_unavailable(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); bin_path = root / 'bin'; bin_path.mkdir()
            (bin_path / 'mkdir').symlink_to(shutil.which('mkdir'))
            env = {'PATH': str(bin_path), 'INVENTORY_ID': 'bootstrap', 'PYTHON_OUTCOME': 'failure'}
            subprocess.run(['/bin/bash', '-euc', script('Preserve terminal evidence independently of Python setup', 'run')], cwd=root, env=env, check=True)
            value = json.loads((root / 'artifacts/python-dependency-inventory/bootstrap/terminal.json').read_text())
            self.assertEqual(value['python'], 'failure'); self.assertEqual(value['payload'], 'skipped')
            self.assertFalse(value['promoted'])


if __name__ == '__main__':
    unittest.main()
