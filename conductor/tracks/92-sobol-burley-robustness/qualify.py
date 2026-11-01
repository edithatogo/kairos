"""Run in repo-local Python with SciPy; argument is an empty scratch directory."""
import json
from pathlib import Path
import subprocess
import sys

import numpy as np
import scipy
from scipy.stats import qmc

root = Path(__file__).resolve().parents[3]
track = Path(__file__).resolve().parent
if len(sys.argv) != 2:
    sys.exit(f"usage: {sys.argv[0]} <empty-scratch-directory>")
scratch = Path(sys.argv[1]).resolve()
if not __debug__:
    raise RuntimeError("qualification requires enabled assertions")
scratch.mkdir(exist_ok=True)
if any(scratch.iterdir()):
    raise ValueError("use an empty scratch directory")
src = scratch / "src"
src.mkdir()
# Pin the historical generator so later main commits cannot change this receipt.
revision = "41e58e0c012e3f2253559cbefb239db71ee7486c"
source_path = "crates/kairo-ecs-cli/src/ensemble.rs"
source = subprocess.check_output(["git", "show", f"{revision}:{source_path}"], cwd=root, text=True)
_, start, tail = source.partition("#[derive(Clone, Debug)]\npub struct SobolSequence")
if not start:
    sys.exit(f"SobolSequence start missing in {revision}:{source_path}")
body, end, _ = tail.partition("/// Deterministic seeded stratified Latin")
if not end:
    sys.exit(f"SobolSequence end missing in {revision}:{source_path}")
(src / "generator.rs").write_text(start + body)
(src / "main.rs").write_bytes((track / "probe.rs").read_bytes())
(scratch / "Cargo.toml").write_text('[workspace]\n[package]\nname="kairos-track92-probe"\nversion="0.0.0"\nedition="2021"\nlicense="Apache-2.0"\npublish=false\n[dependencies]\nsobol_burley={version="=0.5.0",default-features=false}\n')
(scratch / "Cargo.lock").write_bytes((track / "probe.lock").read_bytes())
with (scratch / "points.csv").open("w") as out:
    subprocess.run(["cargo", "run", "--locked", "--quiet"], cwd=scratch, stdout=out, check=True)
rows = np.loadtxt(scratch / "points.csv", delimiter=",")
assert rows.shape == (4096, 7)
assert np.array_equal(rows[:, 0], np.arange(4096))
legacy, candidate, second = rows[:, 1:3], rows[:, 3:5], rows[:, 5:7]
reference = qmc.Sobol(d=2, scramble=False, bits=32).random_base2(12)
assert np.array_equal(reference[:4], [[0, 0], [.5, .5], [.75, .25], [.25, .75]])
assert np.array_equal(legacy[:, 0], reference[:, 0])
assert tuple(legacy[1]) == (.5, 0) and not np.array_equal(legacy, reference)
assert not np.array_equal(candidate, second)
assert np.all((candidate >= 0) & (candidate < 1))
def measurements(points):
    return {str(n): {"occupied_4x4": int(len(np.unique(np.floor(points[:n] * 4), axis=0))),
                     "xy_error": float(abs(np.mean(np.prod(points[:n], axis=1)) - .25))}
            for n in (16, 256, 4096)}
result = {k: measurements(v) for k, v in (("legacy",legacy),("scipy",reference),("candidate",candidate))}
assert result["scipy"]["16"]["occupied_4x4"] == result["candidate"]["16"]["occupied_4x4"] == 16
print(json.dumps({"scipy":scipy.__version__, "dimensions":2, "result":result}, indent=2))
