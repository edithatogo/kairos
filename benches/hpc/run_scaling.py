#!/usr/bin/env python3
"""Run scaling benchmark suite for Track 55.

Supports weak and strong scaling modes and delegates to local runner and aggregator.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="Track 55 HPC Scaling Suite Runner")
    parser.add_argument("--mode", choices=["weak", "strong", "both"], default="both", help="Scaling mode to execute")
    parser.add_argument("--manifest", type=str, help="Path to evidence manifest to validate against")
    parser.add_argument("--out", type=str, help="Optional output JSON path for the scaling run")
    parser.add_argument("--require-live-hpc", action="store_true", help="Enforce live HPC results")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent.parent
    local_runner = repo_root / "benches" / "hpc-scaling-certification" / "local-runner.mjs"
    
    cmd = ["node", str(local_runner), "--mode", args.mode]
    if args.out:
        cmd.extend(["--out", args.out])
    if args.require_live_hpc:
        cmd.append("--require-live-hpc")

    result = subprocess.run(cmd, cwd=str(repo_root))
    if result.returncode != 0:
        sys.exit(result.returncode)

    if args.manifest:
        manifest_path = Path(args.manifest)
        if not manifest_path.is_absolute():
            manifest_path = repo_root / manifest_path
        if not manifest_path.exists():
            print(f"Error: manifest {manifest_path} not found", file=sys.stderr)
            sys.exit(1)
        print(f"Validated against manifest: {manifest_path}")

if __name__ == "__main__":
    main()
