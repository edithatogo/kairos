"""Lock and workflow contracts for the reviewed website and binding fixes."""
from copy import deepcopy
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
PINS = {
    "source-map-js": ("1.2.2", "sha512-KGj/8Y43x35aZVDtt+J4mK1hoLGHULMYfSkODJNQjNDC3oW1PqPoxMwo0pLUsWM/UEGzON/NxeHywEfNXNP3Vw=="),
    "postcss-selector-parser": ("7.1.6", "sha512-7qASPzhKF2l2KLboRZux8CCTRMdGiV08vWmyKzPz22qZ7ZjQBOeY7rNzNoCLSUiftJ7HUq0GERHmxw/t0dCdMw=="),
}


def verify_records(document, package):
    version, integrity = PINS[package]
    records = [value for key, value in document.get("packages", {}).items()
               if key.rsplit("node_modules/", 1)[-1] == package]
    if not records:
        raise ValueError("missing fixed package")
    expected_url = f"https://registry.npmjs.org/{package}/-/{package}-{version}.tgz"
    for record in records:
        if (not isinstance(record, dict) or record.get("version") != version
                or record.get("integrity") != integrity or record.get("resolved") != expected_url
                or ("inBundle" in record and record["inBundle"] is not False)):
            raise ValueError("stale, bundled or unreviewed package")
    return records


class DependencyAdvisoryPinTests(unittest.TestCase):
    def test_fixed_maps_in_website_and_typescript(self):
        for file in ("website/package-lock.json", "bindings/typescript/package-lock.json"):
            with self.subTest(file=file):
                verify_records(json.loads((ROOT / file).read_text()), "source-map-js")

    def test_fixed_website_parser_and_reviewed_consumer_override(self):
        lock = json.loads((ROOT / "website/package-lock.json").read_text())
        manifest = json.loads((ROOT / "website/package.json").read_text())
        verify_records(lock, "postcss-selector-parser")
        self.assertEqual(manifest["overrides"]["@expressive-code/core"]["postcss-nested"], "8.0.1")
        self.assertEqual(manifest["overrides"]["postcss-selector-parser"], "7.1.6")
        self.assertEqual(manifest["overrides"]["source-map-js"], "1.2.2")
        self.assertEqual(lock["packages"]["node_modules/postcss-nested"]["version"], "8.0.1")
        self.assertEqual(lock["packages"]["node_modules/postcss-nested"]["dependencies"]["postcss-selector-parser"], "^7.1.4")

    def test_oracle_rejects_stale_missing_bundled_or_tampered_record(self):
        for package, (version, integrity) in PINS.items():
            record = {"version": version, "integrity": integrity,
                      "resolved": f"https://registry.npmjs.org/{package}/-/{package}-{version}.tgz"}
            key = f"node_modules/{package}"
            base = {"packages": {key: record}}
            self.assertEqual(len(verify_records(base, package)), 1)
            for update in ({"version": "0.0.0"}, {"integrity": "sha512-wrong"},
                           {"resolved": "https://example.invalid/package.tgz"}, {"inBundle": True},
                           {"inBundle": 0}, {"inBundle": "false"}):
                bad = deepcopy(base)
                bad["packages"][key].update(update)
                with self.subTest(package=package, update=update), self.assertRaises(ValueError):
                    verify_records(bad, package)
            with self.assertRaises(ValueError):
                verify_records({"packages": {}}, package)

    def test_oracle_checks_every_nested_copy(self):
        for package, (version, integrity) in PINS.items():
            good = {"version": version, "integrity": integrity,
                    "resolved": f"https://registry.npmjs.org/{package}/-/{package}-{version}.tgz"}
            doc = {"packages": {f"node_modules/{package}": good,
                    f"node_modules/consumer/node_modules/{package}": {**good, "version": "0.0.0"}}}
            with self.subTest(package=package), self.assertRaises(ValueError):
                verify_records(doc, package)

    def test_docs_ci_requires_pin_test_before_install_and_audit(self):
        workflow = (ROOT / ".github/workflows/docs-quality.yml").read_text()
        self.assertIn("      - 'tests/test_dependency_advisory_pins.py'", workflow)
        command = "python3 -m unittest discover -s tests -p test_dependency_advisory_pins.py -v"
        self.assertIn(command, workflow)
        self.assertLess(workflow.index(command), workflow.index("npm ci --prefix website"))
        self.assertLess(workflow.index(command), workflow.index("npm audit --prefix website --audit-level=moderate"))
        self.assertNotIn("continue-on-error", workflow)


if __name__ == "__main__":
    unittest.main()
