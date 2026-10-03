#!/usr/bin/env python3
"""Generate checked SPDX evidence and unsigned provenance for verified archive copies."""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
from email.parser import Parser
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import tomllib
import xml.etree.ElementTree as ET
import zipfile
import uuid
import sys
import importlib.metadata

BUILD_TYPE = 'urn:careops:build-type:verified-archive-copy:v1'
BUILDER_ID = 'urn:careops:local-untrusted-builder:archive-copy'
LIMIT = 2 * 1024**3

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def extract(source: Path, target: Path) -> None:
    target.mkdir()
    names = set()
    total = 0
    def member(name, size):
        nonlocal total
        p = PurePosixPath(name.rstrip('/'))
        if not name or p.is_absolute() or '..' in p.parts or '\\' in name or ':' in name or p.as_posix() != name.rstrip('/'):
            raise ValueError('unsafe member path')
        if p.as_posix() in names:
            raise ValueError('duplicate archive member')
        names.add(p.as_posix())
        total += size
        if total > LIMIT or len(names) > 10000:
            raise ValueError('archive extraction budget exceeded')
        return target / p
    if zipfile.is_zipfile(source):
        with zipfile.ZipFile(source) as z:
            for i in z.infolist():
                p = member(i.filename, i.file_size)
                if i.flag_bits & 1 or stat.S_IFMT(i.external_attr >> 16) not in (0, stat.S_IFREG, stat.S_IFDIR):
                    raise ValueError('encrypted or special archive member')
                if i.is_dir():
                    p.mkdir(parents=True, exist_ok=True)
                else:
                    p.parent.mkdir(parents=True, exist_ok=True)
                    with z.open(i) as src, p.open('xb') as dst:
                        shutil.copyfileobj(src, dst)
    else:
        with tarfile.open(source, 'r:gz') as z:
            for i in z:
                p = member(i.name, i.size)
                if i.isdir():
                    p.mkdir(parents=True, exist_ok=True)
                elif i.isfile():
                    p.parent.mkdir(parents=True, exist_ok=True)
                    with z.extractfile(i) as src, p.open('xb') as dst:
                        shutil.copyfileobj(src, dst)
                else:
                    raise ValueError('special or linked archive member')

def identity(tree: Path, ecosystem: str) -> dict:
    patterns = {'go': 'go.mod', 'julia': 'Project.toml', 'nuget': '*.nuspec',
        'python': 'METADATA', 'r': 'DESCRIPTION', 'rust': 'Cargo.toml', 'typescript': 'package.json'}
    matches = list(tree.rglob(patterns[ecosystem]))
    if ecosystem == 'python' and not matches:
        # Prefer the distribution's root PKG-INFO over vendored nested metadata.
        matches = list(tree.rglob('PKG-INFO'))
    if not matches:
        raise ValueError('missing packaged identity metadata: ' + ecosystem)
    depth = min(len(p.relative_to(tree).parts) for p in matches)
    matches = [p for p in matches if len(p.relative_to(tree).parts) == depth]
    if len(matches) != 1:
        raise ValueError('ambiguous packaged identity metadata: ' + ecosystem)
    p = matches[0]
    text = p.read_text(encoding='utf-8')
    version = None
    repository_commit = None
    if ecosystem == 'go':
        modules = re.findall(r'^module\s+(\S+)\s*$', text, re.M)
        if len(modules) != 1:
            raise ValueError('missing or ambiguous Go module')
        name = modules[0]
    elif ecosystem in ('julia', 'rust'):
        data = tomllib.loads(text)
        data = data['package'] if ecosystem == 'rust' else data
        name, version = data.get('name'), data.get('version')
    elif ecosystem == 'typescript':
        data = json.loads(text)
        name, version = data.get('name'), data.get('version')
    elif ecosystem == 'nuget':
        data = ET.fromstring(text)
        fields = {n.tag.rsplit('}', 1)[-1]: n.text for n in data.iter()}
        name, version = fields.get('id'), fields.get('version')
        repositories = [n for n in data.iter() if n.tag.rsplit('}', 1)[-1] == 'repository']
        if len(repositories) == 1:
            repository_commit = repositories[0].get('commit')
    elif ecosystem == 'python':
        fields = Parser().parsestr(text)
        name, version = fields.get('Name'), fields.get('Version')
    else:
        fields = dict(re.findall(r'^(Package|Version):\s*(.+)$', text, re.M))
        name, version = fields.get('Package'), fields.get('Version')
    if not isinstance(name, str) or not name or (ecosystem != 'go' and (not isinstance(version, str) or not version)):
        raise ValueError('incomplete packaged identity: ' + ecosystem)
    return {'name': name, 'version': version, 'metadata_path': p.relative_to(tree).as_posix(), 'metadata_sha256': digest(p), 'repository_commit': repository_commit}

def load_helper(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def validate_component_graph(document: dict, namespaces: set[str]) -> None:
    namespace = document.get('documentNamespace')
    if not isinstance(namespace, str) or not namespace or namespace in namespaces:
        raise ValueError('missing or duplicate component document namespace')
    # SPDX identifiers are scoped to each document, not globally across SBOMs.
    identifiers = [document.get('SPDXID')]
    for collection in ('packages', 'files', 'snippets'):
        identifiers.extend(item.get('SPDXID') for item in document.get(collection, []))
    if any(not isinstance(identifier, str) or not identifier for identifier in identifiers):
        raise ValueError('missing component SPDX identifier')
    if len(set(identifiers)) != len(identifiers):
        raise ValueError('duplicate SPDX identifier within component document')
    namespaces.add(namespace)

def _generate(source: Path, output: Path, commit: str, acquisition: Path, syft: Path, syft_hash: str, schema: Path, schema_hash: str, acquisition_hash: str, run_id: int) -> None:
    import jsonschema
    if digest(syft) != syft_hash or digest(schema) != schema_hash:
        raise ValueError('tool or SPDX schema identity differs')
    if not re.fullmatch(r'[0-9a-f]{40}', commit) or type(run_id) is not int or run_id <= 0:
        raise ValueError('invalid explicit build identity')
    if digest(acquisition) != acquisition_hash:
        raise ValueError('acquisition receipt identity differs')
    acq = json.loads(acquisition.read_text())
    if acq.get('repository') != 'edithatogo/kairos' or type(acq.get('run_id')) is not int or acq['run_id'] != run_id:
        raise ValueError('acquisition repository or run identity differs')
    if acq.get('source_commit') != commit or not re.fullmatch(r'sha256:[0-9a-f]{64}', acq.get('artifact_digest', '')):
        raise ValueError('acquisition identity differs or lacks artifact digest')
    index = json.loads((source / 'ARCHIVE-INDEX.json').read_text())
    if acq.get('archive_index_sha256') != digest(source / 'ARCHIVE-INDEX.json'):
        raise ValueError('acquisition index digest differs')
    adapter = load_helper('build_archive_release_manifest')
    manifest = adapter.build(source, output, commit)
    started = datetime.now(timezone.utc).isoformat()
    schema_data = json.loads(schema.read_text())
    validator_type = jsonschema.validators.validator_for(schema_data)
    validator_type.check_schema(schema_data)
    validator = validator_type(schema_data, format_checker=jsonschema.FormatChecker())
    retained_inputs = output / 'build-inputs'
    retained_inputs.mkdir()
    for src, name in [(source / 'ARCHIVE-INDEX.json', 'ARCHIVE-INDEX.json'), (source / 'BUILD-RECEIPT.json', 'BUILD-RECEIPT.json'), (acquisition, 'acquisition.json')]:
        shutil.copyfile(src, retained_inputs / name)
    scan_dir = output / 'component-sboms'
    scan_dir.mkdir()
    packages, refs, relationships, coverage = [], [], [], []
    namespaces = set()
    try:
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            config = temporary / 'syft.yaml'
            config.write_text('check-for-app-update: false\nparallelism: 2\ncache:\n  dir: ""\n  ttl: 0\njavascript:\n  search-remote-licenses: false\npython:\n  search-remote-licenses: false\ngolang:\n  search-local-mod-cache-licenses: false\n  search-remote-licenses: false\n  use-packages-lib: false\n')
            env = {'PATH': '/usr/bin:/bin', 'HOME': str(temporary), 'TMPDIR': str(temporary)}
            for row in index['artifacts']:
                short = hashlib.sha256(row['path'].encode()).hexdigest()[:24]
                tree = temporary / short
                archive = output / 'archives' / row['path']
                if digest(archive) != row['sha256'] or archive.stat().st_size != row['bytes']:
                    raise ValueError('delivered archive differs before cataloging')
                extract(archive, tree)
                ident = identity(tree, row['ecosystem'])
                sbom_path = scan_dir / (short + '.spdx.json')
                cmd = [str(syft), '-c', str(config), 'scan', 'dir:' + str(tree), '--source-name', row['path'], '--source-version', commit, '--select-catalogers', '+javascript-package-cataloger', '-o', 'spdx-json=' + str(sbom_path)]
                result = subprocess.run(cmd, capture_output=True, env=env)
                (scan_dir / (short + '.stderr')).write_bytes(result.stderr)
                if result.returncode:
                    raise ValueError('package scan failed: ' + row['path'])
                scan = json.loads(sbom_path.read_text())
                validator.validate(scan)
                validate_component_graph(scan, namespaces)
                found = [p for p in scan.get('packages', []) if p.get('primaryPackagePurpose') != 'FILE']
                accepted_versions = {ident['version']}
                # NuGet nuspec package version omits assembly informational build
                # metadata. Accept only the exact recorded repository commit.
                if row['ecosystem'] == 'nuget' and ident['repository_commit'] == commit:
                    accepted_versions.add(ident['version'] + '+' + commit)
                if row['ecosystem'] not in ('go', 'julia') and not any(p['name'].casefold().replace('_', '-') == ident['name'].casefold().replace('_', '-') and p.get('versionInfo') in accepted_versions for p in found):
                    raise ValueError('scanner did not cover packaged software: ' + row['path'])
                package_id = 'SPDXRef-archive-' + short
                package = {'SPDXID': package_id, 'name': ident['name'], 'packageFileName': 'archives/' + row['path'], 'filesAnalyzed': False, 'downloadLocation': 'NOASSERTION', 'licenseConcluded': 'NOASSERTION', 'licenseDeclared': 'NOASSERTION', 'copyrightText': 'NOASSERTION', 'checksums': [{'algorithm': 'SHA256', 'checksumValue': row['sha256']}], 'sourceInfo': 'Identity from packaged ' + ident['metadata_path'] + ' SHA256 ' + ident['metadata_sha256']}
                if ident['version'] is not None:
                    package['versionInfo'] = ident['version']
                packages.append(package)
                docref = 'DocumentRef-' + short
                refs.append({'externalDocumentId': docref, 'spdxDocument': scan['documentNamespace'], 'checksum': {'algorithm': 'SHA256', 'checksumValue': digest(sbom_path)}})
                relationships.append({'spdxElementId': 'SPDXRef-DOCUMENT', 'relationshipType': 'DESCRIBES', 'relatedSpdxElement': package_id})
                for found_package in found:
                    relationships.append({'spdxElementId': package_id, 'relationshipType': 'CONTAINS', 'relatedSpdxElement': docref + ':' + found_package['SPDXID']})
                coverage.append({'archive': row['path'], 'archive_sha256': row['sha256'], 'identity': ident, 'scanner_software_packages': len(found), 'scanner_identities': [{'name': p['name'], 'version': p.get('versionInfo')} for p in found], 'manifest_identity_fallback': row['ecosystem'] in ('go', 'julia'), 'component_sbom': sbom_path.relative_to(output).as_posix(), 'component_sbom_sha256': digest(sbom_path)})
                if digest(archive) != row['sha256']:
                    raise ValueError('archive changed during cataloging')
        now = datetime.now(timezone.utc).isoformat()
        sbom = {'spdxVersion': 'SPDX-2.3', 'dataLicense': 'CC0-1.0', 'SPDXID': 'SPDXRef-DOCUMENT', 'name': 'Kairos actual package archives', 'documentNamespace': 'https://github.com/edithatogo/kairos/sbom/' + str(uuid.uuid4()), 'creationInfo': {'created': now, 'creators': ['Tool: kairos-archive-evidence']}, 'packages': packages, 'externalDocumentRefs': refs, 'relationships': relationships, 'comment': 'Packaged identities plus detected components. Go/Julia use packaged manifest identities. Unknown licenses/versions remain unknown; this is not a transitive dependency completeness assertion.'}
        validator.validate(sbom)
        (output / 'sbom.spdx.json').write_text(json.dumps(sbom, indent=2, sort_keys=True) + '\n')
        statement = {'_type': 'https://in-toto.io/Statement/v1', 'subject': [{'name': r['path'], 'digest': {'sha256': r['sha256']}} for r in manifest['artifacts']], 'predicateType': 'https://slsa.dev/provenance/v1', 'predicate': {'buildDefinition': {'buildType': BUILD_TYPE, 'externalParameters': {'source_commit': commit, 'original_run_id': acq.get('run_id')}, 'resolvedDependencies': [{'uri': 'https://github.com/edithatogo/kairos/actions/runs/' + str(acq.get('run_id')), 'digest': {'sha256': acq['artifact_digest'][7:]}}, {'uri': 'ARCHIVE-INDEX.json', 'digest': {'sha256': digest(source / 'ARCHIVE-INDEX.json')}}]}, 'runDetails': {'builder': {'id': BUILDER_ID}, 'metadata': {'startedOn': started, 'finishedOn': now}}}}
        source_identities = {name: digest(Path(__file__).with_name(name)) for name in ('build_archive_supply_chain.py', 'build_archive_release_manifest.py', 'build_package_archive_bundle.py')}
        runtime = {'python': sys.version, 'jsonschema': importlib.metadata.version('jsonschema')}
        dependencies = statement['predicate']['buildDefinition']['resolvedDependencies']
        for name in ('ARCHIVE-INDEX.json', 'BUILD-RECEIPT.json', 'acquisition.json'):
            dependencies.append({'uri': 'build-inputs/' + name, 'digest': {'sha256': digest(retained_inputs / name)}})
        for name, value in source_identities.items():
            dependencies.append({'uri': 'packaging/scripts/' + name, 'digest': {'sha256': value}})
        dependencies.extend([{'uri': 'tool:syft', 'digest': {'sha256': syft_hash}}, {'uri': 'schema:spdx-2.3', 'digest': {'sha256': schema_hash}}])
        statement['predicate']['buildDefinition']['internalParameters'] = {'runtime': runtime, 'acquisition_artifact_id': acq.get('artifact_id')}
        (output / 'provenance.json').write_text(json.dumps(statement, indent=2, sort_keys=True) + '\n')
        (output / 'SBOM-COVERAGE.json').write_text(json.dumps({'coverage': coverage, 'source_identities': source_identities, 'runtime': runtime, 'syft_sha256': syft_hash, 'schema_sha256': schema_hash, 'provenance_scope': 'Unsigned local copying/evidence build; does not claim original compilation attestation or SLSA level.'}, indent=2) + '\n')
        evidence_files = sorted(p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_file())
        (output / 'SUPPLY-CHAIN-SHA256SUMS').write_text(''.join(digest(output / p) + '  ' + p + '\n' for p in evidence_files))
        for row in manifest['artifacts']:
            actual = output / row['path']
            if digest(actual) != row['sha256'] or actual.stat().st_size != row['bytes']:
                raise ValueError('delivered archive changed during evidence generation')
        if digest(acquisition) != acquisition_hash or digest(retained_inputs / 'acquisition.json') != acquisition_hash:
            raise ValueError('acquisition receipt changed during generation')
        for name in ('ARCHIVE-INDEX.json', 'BUILD-RECEIPT.json'):
            if digest(source / name) != digest(retained_inputs / name):
                raise ValueError('build input changed during generation')
        if digest(syft) != syft_hash or digest(schema) != schema_hash:
            raise ValueError('tool or schema changed during generation')
    except Exception:
        shutil.rmtree(output)
        raise

def generate(*args, **kwargs):
    output = args[1] if len(args) > 1 else kwargs['output']
    preexisting = output.exists()
    try:
        return _generate(*args, **kwargs)
    except Exception:
        if not preexisting and output.exists():
            shutil.rmtree(output)
        raise

def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('input', 'output', 'acquisition', 'syft', 'spdx-schema'):
        p.add_argument('--' + name, type=Path, required=True)
    for name in ('source-commit', 'syft-sha256', 'spdx-schema-sha256', 'acquisition-sha256'):
        p.add_argument('--' + name, required=True)
    p.add_argument('--run-id', type=int, required=True)
    a = p.parse_args()
    generate(a.input, a.output, a.source_commit, a.acquisition, a.syft, a.syft_sha256, a.spdx_schema, a.spdx_schema_sha256, a.acquisition_sha256, a.run_id)
    print('generated exact-archive SBOM and unsigned copy provenance')
if __name__ == '__main__':
    main()
