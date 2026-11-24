"""Check the resolved build and runtime closure of the standalone Rust fixture."""

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/fixtures/minimal-consumer/Cargo.toml"
MINIMAL_CRATES = {
    "kairo-ecs-minimal-consumer",
    "kairo-ecs-core",
    "kairo-ecs-des",
    "kairo-ecs-rng",
    "kairo-ecs-state",
    "kairo-ecs-types",
}
OPTIONAL_FAMILIES = (
    "adios2",
    "arrow",
    "arrow2",
    "bindgen",
    "careops",
    "cbindgen",
    "cudarc",
    "diplomat",
    "hdf5",
    "http",
    "hyper",
    "innovate",
    "maturin",
    "mpi",
    "ort",
    "paignia",
    "polars",
    "prost",
    "pyo3",
    "reqwest",
    "rsmpi",
    "tokio",
    "tonic",
    "uogto",
    "uniffi",
    "voiage",
    "wasm-bindgen",
    "wgpu",
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", type=Path, default=MANIFEST)
    manifest = parser.parse_args().manifest_path.resolve()
    tree = subprocess.check_output(
        [
            "cargo",
            "tree",
            "--locked",
            "--manifest-path",
            str(manifest),
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ],
        cwd=ROOT,
        text=True,
    )
    packages = {
        line.split()[0].lower().replace("_", "-")
        for line in tree.splitlines()
        if line.strip()
    }
    unexpected = {
        name
        for name in packages
        if (name.startswith("kairo-ecs-") and name not in MINIMAL_CRATES)
        or any(
            name == family or name.startswith(f"{family}-")
            for family in OPTIONAL_FAMILIES
        )
    }
    if unexpected:
        raise SystemExit(
            f"Optional dependencies in the minimal Rust profile: {', '.join(sorted(unexpected))}"
        )
    print(f"Minimal Rust profile: {len(packages)} resolved packages passed.")


if __name__ == "__main__":
    main()
