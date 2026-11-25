"""Read the provider CPU runtime's kernel topology."""

import json
import os
import platform
import subprocess
import sys
from pathlib import Path

root = Path("/sys/devices/system/node")
masks = {name: (root / name).read_text().strip() if (root / name).is_file() else None
         for name in ("online", "has_cpu", "has_memory")}
names = ("Architecture:", "Model name:", "Hypervisor vendor:", "Virtualization type:", "NUMA node")
info = subprocess.run(["lscpu"], check=True, text=True, capture_output=True)
print(json.dumps({
    "system": platform.system(),
    "machine": platform.machine(),
    "kernel": platform.release(),
    "python": sys.version.split()[0],
    "masks": masks,
    "affinity": sorted(os.sched_getaffinity(0)),
    "topology": [line.strip() for line in info.stdout.splitlines() if line.startswith(names)],
}, sort_keys=True), flush=True)
