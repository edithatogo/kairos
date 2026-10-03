import importlib.util
import io
import json
from pathlib import Path
import stat
import tarfile
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("supply_chain", Path(__file__).resolve().parents[1] / "packaging/scripts/build_archive_supply_chain.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class SupplyChainTests(unittest.TestCase):
    def test_zip_rejects_traversal_duplicates_and_links(self):
        for fault in ("traversal", "duplicate", "link"):
            with tempfile.TemporaryDirectory() as d:
                root = Path(d); archive = root / "archive.zip"
                with zipfile.ZipFile(archive, "w") as z:
                    if fault == "traversal": z.writestr("../escape", b"bad")
                    elif fault == "duplicate":
                        z.writestr("same", b"a"); z.writestr("same", b"b")
                    else:
                        i = zipfile.ZipInfo("link"); i.create_system = 3
                        i.external_attr = (stat.S_IFLNK | 0o777) << 16
                        z.writestr(i, "outside")
                with self.assertRaises(ValueError): module.extract(archive, root / "out")
                self.assertFalse((root / "escape").exists())

    def test_tar_rejects_link_and_extraction_budget(self):
        for fault in ("link", "budget"):
            with tempfile.TemporaryDirectory() as d:
                root = Path(d); archive = root / "archive.tar.gz"
                with tarfile.open(archive, "w:gz") as z:
                    i = tarfile.TarInfo("entry")
                    if fault == "link": i.type = tarfile.SYMTYPE; i.linkname = "outside"; z.addfile(i)
                    else: i.size = 4; z.addfile(i, io.BytesIO(b"data"))
                old = module.LIMIT
                try:
                    if fault == "budget": module.LIMIT = 3
                    with self.assertRaises(ValueError): module.extract(archive, root / "out")
                finally: module.LIMIT = old

    def test_go_identity_does_not_invent_version(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); (root / "go.mod").write_text("module example.org/pkg\ngo 1.24\n")
            found = module.identity(root, "go")
            self.assertEqual(found["name"], "example.org/pkg")
            self.assertIsNone(found["version"])
            self.assertEqual(found["metadata_sha256"], module.digest(root / "go.mod"))

    def test_manifest_identity_requires_unique_root_and_version(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "package.json").write_text(json.dumps({"name": "pkg"}))
            with self.assertRaises(ValueError): module.identity(root, "typescript")
            (root / "package.json").unlink()
            for name in ("a", "b"):
                (root / name).mkdir(); (root / name / "package.json").write_text(json.dumps({"name": "pkg", "version": "1"}))
            with self.assertRaises(ValueError): module.identity(root, "typescript")

    def test_acquisition_identity_rejects_wrong_repo_run_and_digest(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); tool = root / "tool"; schema = root / "schema"; acq = root / "acq"
            tool.write_bytes(b"tool"); schema.write_text("{}")
            base = {"repository": "edithatogo/kairos", "run_id": 12, "source_commit": "a" * 40, "artifact_digest": "sha256:" + "b" * 64}
            for change in ({"repository": "other/repo"}, {"run_id": 13}, {"run_id": True}, {"artifact_digest": "missing"}):
                acq.write_text(json.dumps(base | change))
                with self.assertRaises(ValueError):
                    module.generate(root / "absent", root / "output", "a" * 40, acq, tool, module.digest(tool), schema, module.digest(schema), module.digest(acq), 12)
                self.assertFalse((root / "output").exists())

if __name__ == "__main__": unittest.main()
