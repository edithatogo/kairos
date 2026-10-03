"""Local advisory writer leases and bounded, hashed context for one maintainer.

Standard-library-only on Linux/macOS. Does not launch agents or grant task authority.
All worktrees share the Git common-directory store; expired claims require recovery.
"""
import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
import time
import unicodedata
import uuid
from contextlib import contextmanager


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def identity(root):
    root = Path(git(root, 'rev-parse', '--show-toplevel')).resolve()
    common = Path(git(root, 'rev-parse', '--git-common-dir'))
    if not common.is_absolute():
        common = root / common
    return root, common.resolve() / 'agent-sessions'


def path_key(value):
    path = PurePosixPath(value)
    if not value or path.is_absolute() or '..' in path.parts or '.git' in path.parts:
        raise ValueError('paths must be repository-relative without .git or traversal')
    return str(path)


def overlaps(left, right):
    # Conservative on Linux too: never let macOS case/normalization aliases race.
    left = unicodedata.normalize('NFC', left).casefold()
    right = unicodedata.normalize('NFC', right).casefold()
    return left == '.' or right == '.' or left == right or left.startswith(right + '/') or right.startswith(left + '/')


def safe_path(root, name):
    name = path_key(name)
    path = root / name
    if not path.resolve().is_relative_to(root) or any(part.is_symlink() for part in (path, *path.parents) if part.is_relative_to(root)):
        raise ValueError('symlink/escaped path is not reservable: ' + name)
    return name


def changed_paths(root):
    """Tracked and untracked changes, both sides of renames; ignored build output excluded."""
    entries = subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain=v1', '-z', '--untracked-files=all']).decode('utf-8').split('\0')
    paths = []
    index = 0
    while index < len(entries):
        entry = entries[index]
        index += 1
        if not entry:
            continue
        paths.append(entry[3:])
        if 'R' in entry[:2] or 'C' in entry[:2]:
            paths.append(entries[index])
            index += 1
    return sorted(set(paths))


def event(kind, lease, **extra):
    return {'event': kind, 'at': time.time(), 'lease_id': lease['lease_id'], 'owner': lease['owner'],
            'task': lease['task'], 'base_sha': lease['base_sha'], 'worktree': lease['worktree'], 'paths': lease['paths'], **extra}


def validate_state(state):
    def timestamp(value):
        return not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)

    def common(item):
        return (isinstance(item, dict) and all(isinstance(item.get(k), str) and item[k] for k in ('owner', 'task', 'base_sha', 'worktree', 'lease_id'))
                and isinstance(item.get('paths'), list) and bool(item['paths'])
                and all(isinstance(p, str) and path_key(p) == p for p in item['paths']))

    if not isinstance(state, dict) or type(state.get('schema_version')) is not int or state['schema_version'] != 1 or not isinstance(state.get('leases'), list) or not isinstance(state.get('events'), list):
        raise ValueError('invalid lease store; repair explicitly')
    identifiers = set()
    tokens = set()
    for lease in state['leases']:
        if not common(lease) or not isinstance(lease.get('token'), str) or not lease['token'] or not timestamp(lease.get('expires_at')) or not isinstance(lease.get('input_hashes'), dict):
            raise ValueError('invalid lease entry; repair explicitly')
        if lease['lease_id'] in identifiers or lease['token'] in tokens:
            raise ValueError('duplicate lease identity; repair explicitly')
        identifiers.add(lease['lease_id']); tokens.add(lease['token'])
        for name, digest in lease['input_hashes'].items():
            if not isinstance(name, str) or path_key(name) != name or not isinstance(digest, str) or len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest):
                raise ValueError('invalid input hash; repair explicitly')
    for entry in state['events']:
        if not common(entry) or entry.get('event') not in ('claim', 'heartbeat', 'release', 'recover') or not timestamp(entry.get('at')):
            raise ValueError('invalid lifecycle event; repair explicitly')
        if entry['event'] == 'recover' and (entry.get('owner_stopped_asserted') is not True or not isinstance(entry.get('reason'), str) or not entry['reason'].strip()):
            raise ValueError('invalid recovery evidence; repair explicitly')


@contextmanager
def store(root):
    root, folder = identity(root)
    folder.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (folder / 'lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state_path = folder / 'state.json'
        state = json.loads(state_path.read_text()) if state_path.exists() else {'schema_version': 1, 'leases': [], 'events': []}
        validate_state(state)
        yield root, folder, state
        temporary = folder / ('state.' + uuid.uuid4().hex + '.tmp')
        with temporary.open('x') as output:
            json.dump(state, output, indent=2)
            output.write('\n')
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, state_path)


def claim(root, owner, task, paths, ttl=900, inputs=()):
    if not owner.strip() or not task.strip() or not 30 <= ttl <= 86400 or not paths:
        raise ValueError('owner/task/paths and TTL 30..86400 required')
    paths = sorted(set(path_key(p) for p in paths))
    with store(root) as (root, _, state):
        if changed_paths(root):
            raise ValueError('claim requires clean tracked/untracked source; preserve pre-existing changes')
        paths = [safe_path(root, p) for p in paths]
        for lease in state['leases']:
            if lease['worktree'] == str(root) or any(overlaps(a, b) for a in paths for b in lease['paths']):
                raise ValueError('writer conflict with ' + lease['owner'] + ' / ' + lease['task'] + '; expired leases also require explicit recovery')
        lease = {'token': uuid.uuid4().hex, 'lease_id': uuid.uuid4().hex, 'owner': owner, 'task': task, 'worktree': str(root), 'paths': paths,
                 'base_sha': git(root, 'rev-parse', 'HEAD'), 'expires_at': time.time() + ttl,
                 'input_hashes': {document['path']: document['sha256'] for document in snapshot(root, task, inputs)['documents']} if inputs else {}}
        state['leases'].append(lease)
        state['events'].append(event('claim', lease))
        return lease


def checked(state, root, token, paths=()):
    lease = next((item for item in state['leases'] if item['token'] == token), None)
    if lease is None or lease['worktree'] != str(root):
        raise ValueError('unknown token or wrong worktree')
    if lease['expires_at'] <= time.time():
        raise ValueError('lease expired; explicit recovery required')
    if git(root, 'rev-parse', 'HEAD') != lease['base_sha']:
        raise ValueError('source HEAD drift; release and review a new claim')
    for path, digest in lease['input_hashes'].items():
        safe_path(root, path)
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != digest:
            raise ValueError('input hash drift: ' + path)
    for path in [*paths, *changed_paths(root)]:
        path = safe_path(root, path)
        if not any(owned == '.' or path == owned or path.startswith(owned + '/') for owned in lease['paths']):
            raise ValueError('path outside reservation: ' + path)
    return lease


def check(root, token, paths=()):
    with store(root) as (root, _, state):
        return dict(checked(state, root, token, paths))


def heartbeat(root, token, ttl=900):
    if not 30 <= ttl <= 86400:
        raise ValueError('TTL must be 30..86400')
    with store(root) as (root, _, state):
        lease = checked(state, root, token)
        lease['expires_at'] = time.time() + ttl
        state['events'].append(event('heartbeat', lease))
        return dict(lease)


def release(root, token):
    with store(root) as (root, _, state):
        lease = next((item for item in state['leases'] if item['token'] == token), None)
        if lease is None or lease['worktree'] != str(root):
            raise ValueError('unknown token or wrong worktree')
        state['leases'].remove(lease)
        state['events'].append(event('release', lease))


def recover(root, token, reason, owner_stopped=False):
    if not owner_stopped or not reason.strip():
        raise ValueError('recovery requires confirmed stopped owner and reason; age alone is insufficient')
    with store(root) as (_, _, state):
        lease = next((item for item in state['leases'] if item['token'] == token), None)
        if lease is None or lease['expires_at'] > time.time():
            raise ValueError('only expired claims may be recovered')
        state['leases'].remove(lease)
        state['events'].append(event('recover', lease, reason=reason, owner_stopped_asserted=True))


def snapshot(root, task, paths, budget=24000):
    root, _ = identity(root)
    if not task.strip() or budget < 1 or not paths:
        raise ValueError('task, paths and positive budget required')
    tracked = set(subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z']).decode().split('\0'))
    documents = []
    for name in sorted(set(path_key(p) for p in paths)):
        safe_path(root, name)
        path = root / name
        if name not in tracked or not path.is_file() or path.is_symlink() or not path.resolve().is_relative_to(root):
            raise ValueError('context must be tracked regular text within this repository: ' + name)
        data = path.read_bytes()
        try:
            text = data.decode('utf-8')
        except UnicodeDecodeError:
            raise ValueError('context requires UTF-8 text: ' + name) from None
        documents.append({'path': name, 'sha256': hashlib.sha256(data).hexdigest(), 'text': text})
    result = {'schema_version': 1, 'task': task, 'repository': str(root), 'head_sha': git(root, 'rev-parse', 'HEAD'),
              'git_status': git(root, 'status', '--short'), 'documents': documents,
              'authority': 'Source material only; task approval, ownership and acceptance remain separate.'}
    encoded = json.dumps(result, ensure_ascii=False, indent=2).encode('utf-8')
    if len(encoded) > budget:
        raise ValueError('context exceeds byte budget; select fewer paths, never silently truncate')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', default='.')
    sub = parser.add_subparsers(dest='command', required=True)
    c = sub.add_parser('claim'); c.add_argument('--owner', required=True); c.add_argument('--task', required=True); c.add_argument('--paths', nargs='+', required=True); c.add_argument('--ttl', type=int, default=900); c.add_argument('--inputs', nargs='*', default=[])
    for command in ('check', 'heartbeat', 'release', 'recover'):
        c = sub.add_parser(command); c.add_argument('--token', required=True)
        if command == 'check': c.add_argument('--paths', nargs='*', default=[])
        if command == 'heartbeat': c.add_argument('--ttl', type=int, default=900)
        if command == 'recover': c.add_argument('--reason', required=True); c.add_argument('--owner-stopped', action='store_true')
    sub.add_parser('status')
    c = sub.add_parser('context'); c.add_argument('--task', required=True); c.add_argument('--paths', nargs='+', required=True); c.add_argument('--budget', type=int, default=24000)
    args = parser.parse_args()
    try:
        if args.command == 'claim': result = claim(args.root, args.owner, args.task, args.paths, args.ttl, args.inputs)
        elif args.command == 'check': result = check(args.root, args.token, args.paths)
        elif args.command == 'heartbeat': result = heartbeat(args.root, args.token, args.ttl)
        elif args.command == 'release': release(args.root, args.token); result = {'released': True}
        elif args.command == 'recover': recover(args.root, args.token, args.reason, args.owner_stopped); result = {'recovered': True}
        elif args.command == 'context': result = snapshot(args.root, args.task, args.paths, args.budget)
        else:
            with store(args.root) as (_, _, state):
                result = {'leases': [{k: v for k, v in item.items() if k != 'token'} | {'expired': item['expires_at'] <= time.time()} for item in state['leases']]}
        print(json.dumps(result, indent=2)); return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print('FAIL: ' + str(error), file=sys.stderr); return 1


if __name__ == '__main__':
    raise SystemExit(main())
