#!/usr/bin/env python3
"""Bootstrap and verify the selected repository-local Python, without replacement."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import venv

VERSION = (3, 15, 1, 'final', 0)
PROBE = ('import json, platform, sys; '
         'print(json.dumps(dict(version=platform.python_version(), '
         'executable=sys.executable, prefix=sys.prefix)))')


def prepare(root: Path) -> dict[str, str]:
    if tuple(sys.version_info) != VERSION:
        raise RuntimeError('Bootstrap must use exactly Python 3.15.1 (final).')
    root = root.resolve(strict=True)
    target = root / '.venv'
    if target.is_symlink():
        raise RuntimeError('Refusing a symlink environment; preserve and hand over explicitly.')
    if not target.exists():
        venv.EnvBuilder(with_pip=True, symlinks=os.name != 'nt').create(target)
    # An existing environment is only inspected. Never repair/recreate it beneath
    # a consumer, even when it is incomplete or uses another interpreter.
    executable = target / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    try:
        result = subprocess.run([str(executable), '-I', '-c', PROBE],
                                check=True, text=True, capture_output=True)
        receipt = json.loads(result.stdout)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        raise RuntimeError('Local runtime unavailable; preserve it for safe handover.') from error
    if receipt.get('version') != '3.15.1':
        raise RuntimeError('Local runtime must be exactly Python 3.15.1; no replacement attempted.')
    if Path(receipt.get('prefix', '')).resolve() != target:
        raise RuntimeError('Interpreter prefix is not the repository-local environment.')
    receipt['executable'] = str(executable)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--github', action='store_true', help='Export verified local PATH for later steps.')
    args = parser.parse_args()
    try:
        receipt = prepare(args.root)
        if args.github:
            directory = str(Path(receipt['executable']).parent)
            if any(character in directory for character in '\r\n'):
                raise RuntimeError('Cannot export a multiline environment path.')
            with open(os.environ['GITHUB_PATH'], 'a', encoding='utf-8') as output:
                output.write(directory + '\n')
            with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as output:
                output.write('VIRTUAL_ENV=' + receipt['prefix'] + '\n')
                output.write('KAIROS_PYTHON=' + receipt['executable'] + '\n')
        print(json.dumps(receipt, sort_keys=True))
        return 0
    except (RuntimeError, OSError, KeyError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
