import json

import pytest

from notebooks.validate_notebooks import validate_notebook


def write_notebook(path, code_source):
    payload = {
        "cells": [
            {"cell_type": "markdown", "source": "# Validation test"},
            {"cell_type": "code", "source": code_source},
        ],
        "metadata": {"kernelspec": {"name": "python3"}},
        "nbformat": 4,
    }
    path.write_text(json.dumps(payload), encoding="utf-8")


def test_notebook_validation_parses_without_executing_code(tmp_path):
    marker = tmp_path / "cell-was-executed"
    notebook = tmp_path / "side-effect.ipynb"
    write_notebook(
        notebook,
        f"open({str(marker)!r}, 'w', encoding='utf-8').write('executed')",
    )

    validate_notebook(notebook)

    assert not marker.exists()


def test_notebook_syntax_error_includes_path_and_cell_number(tmp_path):
    notebook = tmp_path / "invalid.ipynb"
    write_notebook(notebook, "if True print('invalid')")

    with pytest.raises(AssertionError) as error:
        validate_notebook(notebook)

    assert f"{notebook}:2: Syntax error:" in str(error.value)
