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
import time
import signal

BUILD_TYPE = 'urn:careops:build-type:verified-archive-copy:v1'
BUILDER_ID = 'urn:careops:local-untrusted-builder:archive-copy'
LIMIT = 2 * 1024**3
SCANNER_TIMEOUT_SECONDS = 300
SCANNER_OUTPUT_LIMIT = 8 * 1024**2

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def utc_timestamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec='microseconds').replace('+00:00', 'Z')

def archive_identifier(path: str) -> str:
    return hashlib.sha256(path.encode()).hexdigest()

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

def process_group_helper(expected_sha256: str | None = None):
    """Load exactly the captured acquisition-helper bytes for bounded group checks."""
    name = 'acquire_package_archive_bundle'
    path = Path(__file__).with_name(name + '.py')
    source = path.read_bytes()
    if expected_sha256 is not None and hashlib.sha256(source).hexdigest() != expected_sha256:
        raise ValueError('acquisition helper bytes differ from the recorded source identity')
    module = type(sys)(name)
    module.__file__ = str(path)
    module.__package__ = ''
    exec(compile(source, str(path), 'exec'), module.__dict__)
    return module

def load_provenance_validator():
    name = 'validate_archive_copy_provenance'
    path = Path(__file__).with_name(name + '.py')
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ValueError('cannot load adjacent archive-copy provenance validator')
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module

def validate_component_graph(document: dict, namespaces: set[str], external_documents: dict | None = None) -> None:
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
    external_documents = external_documents or {}
    declared_external = {}
    for reference in document.get('externalDocumentRefs', []):
        identifier = reference.get('externalDocumentId')
        checksum = reference.get('checksum', {})
        digest_value = checksum.get('checksumValue')
        namespace_value = reference.get('spdxDocument')
        if not isinstance(identifier, str) or not identifier or identifier in declared_external:
            raise ValueError('missing or duplicate external document reference')
        if checksum.get('algorithm') != 'SHA256' or not isinstance(digest_value, str) or not re.fullmatch(r'[0-9a-f]{64}', digest_value):
            raise ValueError('invalid external document reference digest')
        target = external_documents.get(identifier)
        if target is None or target.get('sha256') != digest_value or target.get('namespace') != namespace_value:
            raise ValueError('external document reference identity differs')
        declared_external[identifier] = target
    if set(declared_external) != set(external_documents):
        raise ValueError('external document references do not match validated documents')
    local_ids = set(identifiers)
    for relationship in document.get('relationships', []):
        for field in ('spdxElementId', 'relatedSpdxElement'):
            endpoint = relationship.get(field)
            if not isinstance(endpoint, str):
                raise ValueError('invalid SPDX relationship endpoint')
            if endpoint in local_ids or (field == 'relatedSpdxElement' and endpoint in ('NONE', 'NOASSERTION')):
                continue
            if ':' not in endpoint:
                raise ValueError('dangling local SPDX relationship endpoint: ' + endpoint)
            document_id, external_id = endpoint.split(':', 1)
            target = declared_external.get(document_id)
            if target is None or external_id not in target['ids']:
                raise ValueError('dangling external SPDX relationship endpoint: ' + endpoint)
    namespaces.add(namespace)

def run_scanner(command: list[str], env: dict[str, str], stdout_path: Path, stderr_path: Path, sbom_path: Path, helper_sha256: str | None = None) -> int:
    """Run the scanner with a time limit and bounded, retained output files."""
    # Bind cleanup behavior to helper bytes loaded for this scanner invocation.
    # Loading before Popen also ensures helper failures cannot orphan a child.
    helper = process_group_helper(helper_sha256)
    started = time.monotonic()
    with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, env=env, start_new_session=True)
        def group_exists() -> bool:
            return helper._process_group_exists(process.pid)
        def stop_group() -> bool:
            if not group_exists():
                return False
            helper._signal_process_group(process, signal.SIGTERM)
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                process.poll()  # Reap the direct child so its zombie cannot keep the group visible.
                if not group_exists():
                    return True
                time.sleep(0.05)
            helper._signal_process_group(process, signal.SIGKILL)
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                process.poll()
                if not group_exists():
                    return True
                time.sleep(0.05)
            raise ValueError('package scanner process group did not stop')
        try:
            while process.poll() is None:
                sbom_bytes = sbom_path.stat().st_size if sbom_path.exists() else 0
                if max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size, sbom_bytes) > SCANNER_OUTPUT_LIMIT:
                    raise ValueError('package scanner output limit exceeded')
                if time.monotonic() - started > SCANNER_TIMEOUT_SECONDS:
                    raise ValueError('package scanner timed out')
                time.sleep(0.05)
            sbom_bytes = sbom_path.stat().st_size if sbom_path.exists() else 0
            if max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size, sbom_bytes) > SCANNER_OUTPUT_LIMIT:
                raise ValueError('package scanner output limit exceeded')
            result = process.returncode
            lingering_child = stop_group()
            sbom_bytes = sbom_path.stat().st_size if sbom_path.exists() else 0
            if max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size, sbom_bytes) > SCANNER_OUTPUT_LIMIT:
                raise ValueError('package scanner output limit exceeded')
            if lingering_child:
                raise ValueError('package scanner left a child process running')
            if result:
                tail = stderr_path.read_bytes()[-4096:].decode('utf-8', errors='replace')
                raise ValueError(f'package scanner failed with exit status {result}; stderr tail={tail!r}')
            return result
        except BaseException:
            stop_group()
            if process.poll() is None:
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    helper._signal_process_group(process, signal.SIGKILL)
                    process.wait(timeout=2)
            raise

def _generate(source: Path, output: Path, commit: str, acquisition: Path, syft: Path, syft_hash: str, schema: Path, schema_hash: str, acquisition_hash: str, run_id: int) -> None:
    helper_names = (
        'build_archive_supply_chain.py',
        'build_archive_release_manifest.py',
        'build_package_archive_bundle.py',
        'acquire_package_archive_bundle.py',
        'validate_archive_copy_provenance.py',
    )
    helper_paths = {name: Path(__file__).with_name(name) for name in helper_names}
    source_identities = {name: digest(path) for name, path in helper_paths.items()}
    provenance_validator = load_provenance_validator()
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
    if type(acq.get('artifact_id')) is not int or acq['artifact_id'] <= 0:
        raise ValueError('acquisition artifact ID must be a positive integer')
    source_index_bytes = (source / 'ARCHIVE-INDEX.json').read_bytes()
    pinned_index_sha256 = hashlib.sha256(source_index_bytes).hexdigest()
    index = json.loads(source_index_bytes)
    if acq.get('archive_index_sha256') != pinned_index_sha256:
        raise ValueError('acquisition index digest differs')
    identifiers_by_path = {}
    package_ids, document_ids = set(), set()
    for row in index['artifacts']:
        if row['path'] in identifiers_by_path:
            raise ValueError('duplicate archive path in index')
        identifier = archive_identifier(row['path'])
        package_id = 'SPDXRef-archive-' + identifier
        document_id = 'DocumentRef-' + identifier
        if package_id in package_ids or document_id in document_ids:
            raise ValueError('archive identifier collision')
        package_ids.add(package_id)
        document_ids.add(document_id)
        identifiers_by_path[row['path']] = identifier
    adapter = load_helper('build_archive_release_manifest')
    manifest = adapter.build(source, output, commit)
    started = utc_timestamp()
    schema_data = json.loads(schema.read_text())
    validator_type = jsonschema.validators.validator_for(schema_data)
    validator_type.check_schema(schema_data)
    validator = validator_type(schema_data, format_checker=jsonschema.FormatChecker())
    retained_inputs = output / 'build-inputs'
    retained_inputs.mkdir()
    (retained_inputs / 'ARCHIVE-INDEX.json').write_bytes(source_index_bytes)
    for src, name in [(source / 'BUILD-RECEIPT.json', 'BUILD-RECEIPT.json'), (acquisition, 'acquisition.json')]:
        shutil.copyfile(src, retained_inputs / name)
    if digest(retained_inputs / 'ARCHIVE-INDEX.json') != pinned_index_sha256:
        raise ValueError('retained archive index differs from the initial source index bytes')
    scan_dir = output / 'component-sboms'
    scan_dir.mkdir()
    packages, refs, relationships, coverage = [], [], [], []
    component_documents = {}
    namespaces = set()
    try:
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            config = temporary / 'syft.yaml'
            config.write_text('check-for-app-update: false\nparallelism: 2\ncache:\n  dir: ""\n  ttl: 0\njavascript:\n  search-remote-licenses: false\npython:\n  search-remote-licenses: false\ngolang:\n  search-local-mod-cache-licenses: false\n  search-remote-licenses: false\n  use-packages-lib: false\n')
            env = {'PATH': '/usr/bin:/bin', 'TMPDIR': str(temporary)}
            for row in index['artifacts']:
                short = identifiers_by_path[row['path']]
                package_id = 'SPDXRef-archive-' + short
                docref = 'DocumentRef-' + short
                tree = temporary / short
                archive = output / 'archives' / row['path']
                if digest(archive) != row['sha256'] or archive.stat().st_size != row['bytes']:
                    raise ValueError('delivered archive differs before cataloging')
                extract(archive, tree)
                ident = identity(tree, row['ecosystem'])
                sbom_path = scan_dir / (short + '.spdx.json')
                cmd = [str(syft), '-c', str(config), 'scan', 'dir:' + str(tree), '--source-name', row['path'], '--source-version', commit, '--select-catalogers', '+javascript-package-cataloger', '-o', 'spdx-json=' + str(sbom_path)]
                scan_status = run_scanner(cmd, env, scan_dir / (short + '.stdout'), scan_dir / (short + '.stderr'), sbom_path, source_identities['acquire_package_archive_bundle.py'])
                if scan_status:
                    raise ValueError('package scan failed: ' + row['path'])
                scan = json.loads(sbom_path.read_text())
                validator.validate(scan)
                validate_component_graph(scan, namespaces)
                component_ids = {scan['SPDXID']}
                for collection in ('packages', 'files', 'snippets'):
                    component_ids.update(item['SPDXID'] for item in scan.get(collection, []))
                found = [p for p in scan.get('packages', []) if p.get('primaryPackagePurpose') != 'FILE']
                accepted_versions = {ident['version']}
                # NuGet nuspec package version omits assembly informational build
                # metadata. Accept only the exact recorded repository commit.
                if row['ecosystem'] == 'nuget' and ident['repository_commit'] == commit:
                    accepted_versions.add(ident['version'] + '+' + commit)
                if row['ecosystem'] not in ('go', 'julia') and not any(p['name'].casefold().replace('_', '-') == ident['name'].casefold().replace('_', '-') and p.get('versionInfo') in accepted_versions for p in found):
                    raise ValueError('scanner did not cover packaged software: ' + row['path'])
                package = {'SPDXID': package_id, 'name': ident['name'], 'packageFileName': 'archives/' + row['path'], 'filesAnalyzed': False, 'downloadLocation': 'NOASSERTION', 'licenseConcluded': 'NOASSERTION', 'licenseDeclared': 'NOASSERTION', 'copyrightText': 'NOASSERTION', 'checksums': [{'algorithm': 'SHA256', 'checksumValue': row['sha256']}], 'sourceInfo': 'Identity from packaged ' + ident['metadata_path'] + ' SHA256 ' + ident['metadata_sha256']}
                if ident['version'] is not None:
                    package['versionInfo'] = ident['version']
                packages.append(package)
                refs.append({'externalDocumentId': docref, 'spdxDocument': scan['documentNamespace'], 'checksum': {'algorithm': 'SHA256', 'checksumValue': digest(sbom_path)}})
                component_documents[docref] = {'namespace': scan['documentNamespace'], 'sha256': digest(sbom_path), 'ids': component_ids}
                relationships.append({'spdxElementId': 'SPDXRef-DOCUMENT', 'relationshipType': 'DESCRIBES', 'relatedSpdxElement': package_id})
                for found_package in found:
                    relationships.append({'spdxElementId': package_id, 'relationshipType': 'CONTAINS', 'relatedSpdxElement': docref + ':' + found_package['SPDXID']})
                coverage.append({'archive': row['path'], 'archive_sha256': row['sha256'], 'identity': ident, 'scanner_software_packages': len(found), 'scanner_identities': [{'name': p['name'], 'version': p.get('versionInfo')} for p in found], 'manifest_identity_fallback': row['ecosystem'] in ('go', 'julia'), 'component_sbom': sbom_path.relative_to(output).as_posix(), 'component_sbom_sha256': digest(sbom_path)})
                if digest(archive) != row['sha256']:
                    raise ValueError('archive changed during cataloging')
        now = utc_timestamp()
        sbom = {'spdxVersion': 'SPDX-2.3', 'dataLicense': 'CC0-1.0', 'SPDXID': 'SPDXRef-DOCUMENT', 'name': 'Kairos actual package archives', 'documentNamespace': 'https://github.com/edithatogo/kairos/sbom/' + str(uuid.uuid4()), 'creationInfo': {'created': now, 'creators': ['Tool: kairos-archive-evidence']}, 'packages': packages, 'externalDocumentRefs': refs, 'relationships': relationships, 'comment': 'Packaged identities plus detected components. Go/Julia use packaged manifest identities. Unknown licenses/versions remain unknown; this is not a transitive dependency completeness assertion.'}
        validator.validate(sbom)
        validate_component_graph(sbom, set(), component_documents)
        (output / 'sbom.spdx.json').write_text(json.dumps(sbom, indent=2, sort_keys=True) + '\n')
        runtime = {'python': sys.version, 'jsonschema': importlib.metadata.version('jsonschema')}
        dependency_inputs = [
            {'id': 'https://github.com/edithatogo/kairos/actions/runs/' + str(run_id), 'sha256': acq['artifact_digest'][7:]},
            {'id': 'ARCHIVE-INDEX.json', 'sha256': pinned_index_sha256},
            {'id': 'build-inputs/ARCHIVE-INDEX.json', 'sha256': digest(retained_inputs / 'ARCHIVE-INDEX.json')},
        ]
        for name in ('BUILD-RECEIPT.json', 'acquisition.json'):
            dependency_inputs.append({'id': 'build-inputs/' + name, 'sha256': digest(retained_inputs / name)})
        for name, value in source_identities.items():
            dependency_inputs.append({'id': 'packaging/scripts/' + name, 'sha256': value})
        dependency_inputs.extend([
            {'id': 'tool:syft', 'sha256': syft_hash},
            {'id': 'schema:spdx-2.3', 'sha256': schema_hash},
        ])
        expected_inputs = {
            'archive_index_sha256': pinned_index_sha256,
            'source_commit': commit,
            'original_run_id': run_id,
            'acquisition_artifact_id': acq['artifact_id'],
            'dependencies': dependency_inputs,
        }
        dependencies = [
            {
                'name': item['id'],
                'uri': provenance_validator.canonical_dependency_uri(item['id'], run_id),
                'digest': {'sha256': item['sha256']},
            }
            for item in dependency_inputs
        ]
        statement = {
            '_type': 'https://in-toto.io/Statement/v1',
            'subject': [{'name': r['path'], 'digest': {'sha256': r['sha256']}} for r in manifest['artifacts']],
            'predicateType': 'https://slsa.dev/provenance/v1',
            'predicate': {'buildDefinition': {
                'buildType': BUILD_TYPE,
                'externalParameters': {'source_commit': commit, 'original_run_id': run_id},
                'resolvedDependencies': dependencies,
            }, 'runDetails': {'builder': {'id': BUILDER_ID}, 'metadata': {'startedOn': started, 'finishedOn': now}}},
        }
        statement['predicate']['buildDefinition']['internalParameters'] = {'runtime': runtime, 'acquisition_artifact_id': acq.get('artifact_id')}
        expected_path = output / 'expected-inputs.json'
        expected_serialized = (json.dumps(expected_inputs, indent=2, sort_keys=True) + '\n').encode('utf-8')
        expected_path.write_bytes(expected_serialized)
        statement_path = output / 'provenance.json'
        statement_path.write_text(json.dumps(statement, indent=2, sort_keys=True) + '\n')
        for name, path in helper_paths.items():
            if digest(path) != source_identities[name]:
                raise ValueError('provenance helper source changed before validation: ' + name)
        loaded_statement, statement_bytes = provenance_validator.load_json_file(statement_path, 'provenance statement')
        loaded_index, index_bytes = provenance_validator.load_json_file(retained_inputs / 'ARCHIVE-INDEX.json', 'retained archive index')
        loaded_expected, expected_bytes = provenance_validator.load_json_file(expected_path, 'independent expected inputs')
        if expected_bytes != expected_serialized or loaded_expected != expected_inputs:
            raise ValueError('serialized expected inputs differ from independently assembled inputs')
        validation_issues = provenance_validator.validate_provenance(
            loaded_statement,
            loaded_index,
            expected_inputs,
            hashlib.sha256(index_bytes).hexdigest(),
        )
        validation_result = {
            'valid': not validation_issues,
            'statement_sha256': hashlib.sha256(statement_bytes).hexdigest(),
            'archive_index_sha256': hashlib.sha256(index_bytes).hexdigest(),
            'expected_inputs_sha256': hashlib.sha256(expected_bytes).hexdigest(),
            'validator_sha256': source_identities['validate_archive_copy_provenance.py'],
            'issue_count': len(validation_issues),
            'issues': [issue.as_dict() for issue in validation_issues],
            'claim_scope': 'local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance',
        }
        (output / 'validation-result.json').write_text(json.dumps(validation_result, indent=2, sort_keys=True) + '\n')
        if validation_issues:
            raise ValueError('archive-copy provenance validation failed: ' + ', '.join(issue.code for issue in validation_issues[:8]))
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
        if digest(source / 'ARCHIVE-INDEX.json') != pinned_index_sha256:
            raise ValueError('source archive index changed during generation')
        if digest(syft) != syft_hash or digest(schema) != schema_hash:
            raise ValueError('tool or schema changed during generation')
        for name, path in helper_paths.items():
            if digest(path) != source_identities[name]:
                raise ValueError('provenance helper source changed during generation: ' + name)
    except BaseException:
        raise

def generate(*args, **kwargs):
    output = args[1] if len(args) > 1 else kwargs['output']
    preexisting = output.exists()
    try:
        return _generate(*args, **kwargs)
    except BaseException:
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
