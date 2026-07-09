from __future__ import annotations

import ast
import json
import os
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
NOTEBOOK_ROOT = REPO_ROOT / "notebooks"
FORBIDDEN_CODE_PATTERNS = (
    "requests.",
    "urllib.",
    "socket.",
    "subprocess",
    "pip install",
    "%pip",
    "!pip",
    "!python",
    "!powershell",
    "!pwsh",
)


def main() -> None:
    notebooks = sorted(
        path
        for path in NOTEBOOK_ROOT.rglob("*.ipynb")
        if ".ipynb_checkpoints" not in path.parts
    )
    if not notebooks:
        raise SystemExit("No notebooks found")

    previous_cwd = Path.cwd()
    os.chdir(REPO_ROOT)
    try:
        for notebook in notebooks:
            validate_notebook(notebook)
    finally:
        os.chdir(previous_cwd)

    print(f"notebook_validation=ok notebooks={len(notebooks)}")


def validate_notebook(path: Path) -> None:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("nbformat") != 4:
        raise AssertionError(f"{path}: expected nbformat 4")

    cells = payload.get("cells")
    if not isinstance(cells, list) or not cells:
        raise AssertionError(f"{path}: notebook has no cells")

    kernelspec = payload.get("metadata", {}).get("kernelspec", {})
    if kernelspec.get("name") != "python3":
        raise AssertionError(f"{path}: expected python3 kernelspec")

    is_colab_smoke = path.name.startswith("colab_")
    code_cells = 0
    markdown_cells = 0
    for index, cell in enumerate(cells, start=1):
        cell_type = cell.get("cell_type")
        source = source_text(cell.get("source", ""))
        if cell_type == "markdown":
            markdown_cells += 1
            validate_local_images(path, source)
        elif cell_type == "code":
            code_cells += 1
            validate_code_source(path, index, source, allow_notebook_magics=is_colab_smoke)
            if not is_colab_smoke:
                try:
                    ast.parse(source, filename=f"{path}:{index}")
                except SyntaxError as e:
                    raise AssertionError(f"{path}:{index}: Syntax error: {e}") from e
        else:
            raise AssertionError(f"{path}: unsupported cell type {cell_type!r}")

    if markdown_cells == 0 or code_cells == 0:
        raise AssertionError(f"{path}: expected markdown and code cells")


def source_text(source: object) -> str:
    if isinstance(source, list):
        return "".join(str(part) for part in source)
    return str(source)


def validate_code_source(path: Path, index: int, source: str, *, allow_notebook_magics: bool = False) -> None:
    lowered = source.lower()
    for pattern in FORBIDDEN_CODE_PATTERNS:
        if allow_notebook_magics and pattern in {"pip install", "%pip", "!pip", "!python", "!powershell", "!pwsh"}:
            continue
        if pattern in lowered:
            raise AssertionError(f"{path}:{index}: forbidden code pattern {pattern!r}")


def validate_local_images(path: Path, source: str) -> None:
    for target in re.findall(r"!\[[^\]]*\]\(([^)]+)\)", source):
        if re.match(r"^[a-z]+://", target):
            raise AssertionError(f"{path}: remote image reference is not allowed: {target}")
        image_path = (path.parent / target).resolve()
        if not image_path.is_file():
            raise AssertionError(f"{path}: missing local image reference: {target}")
        if NOTEBOOK_ROOT not in image_path.parents:
            raise AssertionError(f"{path}: image reference escapes notebooks/: {target}")


if __name__ == "__main__":
    main()
