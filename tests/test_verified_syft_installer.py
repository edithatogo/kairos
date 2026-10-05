from __future__ import annotations

import hashlib
import gzip
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
from unittest import mock

from scripts.supply_chain import install_verified_syft as syft


class VerifiedSyftInstallerTests(unittest.TestCase):
    def test_only_native_linux_amd64_and_macos_arm64_targets_are_supported(self):
        darwin = syft.detect_target("Darwin", "arm64")
        linux = syft.detect_target("Linux", "x86_64")
        self.assertEqual(darwin["asset"], "syft_1.54.0_darwin_arm64.tar.gz")
        self.assertEqual(darwin["binary_sha256"], "835607cdfbdbfc59335b0beadeefc47aa6aab7d3b403c11cfa65627d92a27f61")
        self.assertEqual(linux["asset"], "syft_1.54.0_linux_amd64.tar.gz")
        self.assertEqual(linux["binary_sha256"], "d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92")
        with self.assertRaisesRegex(RuntimeError, "unsupported host"):
            syft.detect_target("Darwin", "x86_64")
        with self.assertRaisesRegex(RuntimeError, "unsupported host"):
            syft.detect_target("Linux", "aarch64")
        self.assertNotIn("--target", sys.argv)

    def test_platform_verifier_locks_are_exact_retained_bytes(self):
        root = Path(__file__).resolve().parents[1] / "scripts/supply_chain"
        linux = root / "syft-linux-verifier.lock"
        darwin = root / "syft-darwin-verifier.lock"
        self.assertEqual(syft.sha256_file(linux), "e8c2913539b2dc4260ef8611e1f21daa56efbdf8b55199c34882808aecd6acea")
        self.assertEqual(syft.sha256_file(darwin), "bc22323572381258237ff65529b55f37387a3bdfddf1ccf82305d41443ddacf2")
        for target in (syft.detect_target("Linux", "x86_64"), syft.detect_target("Darwin", "arm64")):
            path, raw = syft.validate_lock(target, root)
            self.assertTrue(path.is_file())
            self.assertIn(b"sigstore==4.5.0", raw)
            self.assertIn(b"pypi-attestations==0.0.30", raw)

    def test_lock_digest_and_rows_fail_closed(self):
        target = syft.detect_target("Darwin", "arm64")
        with tempfile.TemporaryDirectory() as temporary:
            lock_dir = Path(temporary).resolve()
            (lock_dir / target["verifier_lock"]).write_text("sigstore==4.5.0 --hash=sha256:" + "0" * 64 + "\n", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "lock digest mismatch"):
                syft.validate_lock(target, lock_dir)
            target["verifier_lock_sha256"] = syft.sha256_file(lock_dir / target["verifier_lock"])
            with self.assertRaisesRegex(RuntimeError, "must retain"):
                syft.validate_lock(target, lock_dir)

    def test_checksum_parser_requires_one_well_formed_target_row(self):
        asset = "syft_1.54.0_darwin_arm64.tar.gz"
        digest = "a" * 64
        self.assertEqual(syft.Installer._parse_checksum(f"{digest}  {asset}\n".encode(), asset), digest)
        for raw in [b"", f"{digest} {asset}\n{digest} {asset}\n".encode(), f"{digest} {asset}\nBAD\n".encode()]:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                syft.Installer._parse_checksum(raw, asset)

    def test_version_probe_requires_exact_identity_and_strict_json(self):
        expected = {"application": "syft", "version": syft.VERSION,
                    "gitCommit": syft.RELEASE_COMMIT, "platform": "darwin/arm64"}
        self.assertEqual(syft.validate_version_output(json.dumps(expected), "darwin/arm64"), expected)
        for field, value in (("application", "other"), ("version", "9.9.9"),
                             ("gitCommit", "0" * 40), ("platform", "linux/amd64")):
            altered = dict(expected)
            altered[field] = value
            with self.subTest(field=field), self.assertRaisesRegex(RuntimeError, "does not match"):
                syft.validate_version_output(json.dumps(altered), "darwin/arm64")
        for malformed in ('{"application":"syft","application":"other"}',
                          '{"application":"syft","value":1e999}', "not-json"):
            with self.subTest(output=malformed), self.assertRaisesRegex(RuntimeError, "strict JSON"):
                syft.validate_version_output(malformed, "darwin/arm64")

    def test_download_urls_and_redirects_are_host_and_scheme_pinned(self):
        syft._checked_url(syft.CHECKSUM_URL)
        with self.assertRaisesRegex(RuntimeError, "unpinned"):
            syft._checked_url("http://github.com/anchore/syft/releases/download/v1.54.0/x")
        with self.assertRaisesRegex(RuntimeError, "unpinned"):
            syft._checked_url("https://evil.invalid/syft_1.54.0_checksums.txt")
        for url in (syft.CHECKSUM_URL.replace("https://", "https://user@"),
                    syft.CHECKSUM_URL.replace("github.com", "github.com:444")):
            with self.subTest(url=url), self.assertRaisesRegex(RuntimeError, "unpinned"):
                syft._checked_url(url)
        handler = syft._ReleaseRedirectHandler()
        from urllib.request import Request
        class Response:
            status = 302
            headers = {}
        request = Request(syft.CHECKSUM_URL)
        self.assertIsNotNone(handler.redirect_request(request, Response(), 302, "found", {}, "https://release-assets.githubusercontent.com/signed/object"))
        with self.assertRaisesRegex(RuntimeError, "cross-origin"):
            handler.redirect_request(request, Response(), 302, "found", {}, "http://release-assets.githubusercontent.com/x")
        with self.assertRaisesRegex(RuntimeError, "cross-origin"):
            handler.redirect_request(request, Response(), 302, "found", {}, "https://attacker.invalid/x")
        loop = syft._ReleaseRedirectHandler()
        for _ in range(4):
            self.assertIsNotNone(loop.redirect_request(request, Response(), 302, "found", {}, syft.CHECKSUM_URL))
        with self.assertRaisesRegex(RuntimeError, "redirect limit"):
            loop.redirect_request(request, Response(), 302, "found", {}, syft.CHECKSUM_URL)

    def test_fetch_rejects_unexpected_content_type_without_writing(self):
        class Response:
            status = 200
            headers = {"Content-Type": "text/html", "Content-Length": "1"}
            def geturl(self):
                return syft.CHECKSUM_URL
            def read(self, size=-1):
                return b"x"
            def __enter__(self):
                return self
            def __exit__(self, *args):
                return None
        class Opener:
            def open(self, request, timeout):
                return Response()
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary).resolve() / "download"
            with mock.patch.object(syft, "require_supported_python"), \
                 mock.patch.object(syft.urllib.request, "build_opener", return_value=Opener()):
                with self.assertRaisesRegex(RuntimeError, "Content-Type"):
                    syft._fetch_to_file(syft.CHECKSUM_URL, destination, 16)
            self.assertFalse(destination.exists())

    def _make_archive(self, path: Path, entries: dict[str, bytes], *, types: dict[str, bytes] | None = None) -> None:
        with tarfile.open(path, "w:gz") as archive:
            for name, content in entries.items():
                info = tarfile.TarInfo(name)
                info.size = len(content)
                info.mode = 0o755 if name == "syft" else 0o644
                info.mtime = 0
                info.type = (types or {}).get(name, tarfile.REGTYPE)
                archive.addfile(info, io.BytesIO(content))

    def test_safe_extraction_hashes_binary_and_rejects_unsafe_members(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            archive = root / "safe.tar.gz"
            self._make_archive(archive, {"CHANGELOG.md": b"changelog", "LICENSE": b"license",
                                         "README.md": b"readme", "syft": b"binary"})
            installer = syft.Installer(root / "unused", system="Darwin", machine="arm64")
            extracted, receipt = installer._safe_extract(archive, root / "safe-out")
            self.assertEqual(extracted.read_bytes(), b"binary")
            self.assertEqual(receipt["binary_sha256"], hashlib.sha256(b"binary").hexdigest())
            with self.assertRaisesRegex(ValueError, "authenticated digest check"):
                installer._safe_extract(archive, root / "wrong-digest-out", "0" * 64)
            self.assertFalse((root / "wrong-digest-out").exists())
            cases = [
                ({"../escape": b"x", "syft": b"bin"}, None),
                ({"/absolute": b"x", "syft": b"bin"}, None),
                ({"C:/drive": b"x", "syft": b"bin"}, None),
                ({"syft": b"bin"}, {"syft": tarfile.SYMTYPE}),
                ({"syft": b"bin"}, {"syft": tarfile.LNKTYPE}),
                ({"syft": b"bin"}, {"syft": tarfile.FIFOTYPE}),
            ]
            for number, (entries, types) in enumerate(cases):
                archive = root / f"bad-{number}.tar.gz"
                self._make_archive(archive, entries, types=types)
                destination = root / f"bad-out-{number}"
                with self.subTest(case=number), self.assertRaises((ValueError, OSError)):
                    installer._safe_extract(archive, destination)
                self.assertFalse(destination.exists())

    def test_safe_extraction_rejects_duplicate_canonical_and_unexpected_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            cases = [("unexpected", ["unexpected"]), ("dot-component", ["./syft"]),
                     ("duplicate-canonical", ["syft", "syft"])]
            for name, member_names in cases:
                archive = root / f"{name.replace('/', '_')}.tar.gz"
                with tarfile.open(archive, "w:gz") as tar:
                    for member_name in member_names:
                        info = tarfile.TarInfo(member_name)
                        info.size = 1
                        tar.addfile(info, io.BytesIO(b"x"))
                installer = syft.Installer(root / f"unused-{len(name)}", system="Darwin", machine="arm64")
                with self.assertRaises(ValueError):
                    installer._safe_extract(archive, root / f"out-{len(name)}")

    def test_archive_must_contain_exact_four_regular_profile_members(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            installer = syft.Installer(root / "unused", system="Darwin", machine="arm64")
            for entries in (
                {"LICENSE": b"license", "syft": b"binary"},
                {"CHANGELOG.md": b"changelog", "LICENSE": b"license", "README.md": b"readme",
                 "syft": b"binary", "unexpected": b"extra"},
            ):
                archive = root / f"archive-{len(entries)}.tar.gz"
                self._make_archive(archive, entries)
                with self.assertRaises(ValueError):
                    installer._safe_extract(archive, root / f"extract-{len(entries)}")

    def test_strict_json_rejects_duplicate_keys_nonfinite_values_and_excess_depth(self):
        self.assertEqual(syft.strict_json(b'{"a":1}', label="test"), {"a": 1})
        for raw in (b'{"a":1,"a":2}', b'{"a":1e999}', b'{"a":NaN}', b"[[[[[0]]]]]"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                syft.strict_json(raw, label="test", depth_limit=4)

    def test_python_version_is_checked_before_output_creation_or_fetch(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary).resolve() / "output"
            installer = syft.Installer(output, system="Darwin", machine="arm64")
            with mock.patch.object(syft, "require_supported_python", side_effect=RuntimeError("wrong Python")), \
                 mock.patch.object(syft, "validate_lock") as validate:
                with self.assertRaisesRegex(RuntimeError, "wrong Python"):
                    installer.run()
            validate.assert_not_called()
            self.assertFalse(output.exists())

    def test_lock_rejects_symlinked_file_and_ancestor(self):
        target = syft.detect_target("Darwin", "arm64")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            real = root / "real"
            real.mkdir()
            lock = real / target["verifier_lock"]
            lock.write_bytes(b"fixture")
            target["verifier_lock_sha256"] = hashlib.sha256(b"fixture").hexdigest()
            link_file_dir = root / "file-link"
            link_file_dir.mkdir()
            (link_file_dir / target["verifier_lock"]).symlink_to(lock)
            with self.assertRaisesRegex(RuntimeError, "regular non-symlink"):
                syft.validate_lock(target, link_file_dir)
            link_parent = root / "parent-link"
            link_parent.symlink_to(real, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, "ancestry contains a symlink"):
                syft.validate_lock(target, link_parent)

    def test_lock_ancestor_swap_to_symlink_is_rejected_by_descriptor_walk(self):
        target = syft.detect_target("Darwin", "arm64")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            parent = root / "lock-parent"
            parent.mkdir()
            lock = parent / target["verifier_lock"]
            lock.write_bytes(b"fixture")
            target["verifier_lock_sha256"] = hashlib.sha256(b"fixture").hexdigest()
            moved = root / "lock-parent-held"
            original_open = os.open
            swapped = False
            def swap_then_open(path, flags, *args, **kwargs):
                nonlocal swapped
                if path == "lock-parent" and kwargs.get("dir_fd") is not None and not swapped:
                    parent.rename(moved)
                    parent.symlink_to(moved, target_is_directory=True)
                    swapped = True
                return original_open(path, flags, *args, **kwargs)
            with mock.patch.object(syft.os, "open", side_effect=swap_then_open):
                with self.assertRaisesRegex(RuntimeError, "symlink"):
                    syft.validate_lock(target, parent)
            self.assertTrue(swapped)

    def test_fifo_lock_is_rejected_without_blocking(self):
        target = syft.detect_target("Darwin", "arm64")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            os.mkfifo(root / target["verifier_lock"])
            with self.assertRaisesRegex(RuntimeError, "regular file"):
                syft.validate_lock(target, root)

    def test_output_and_parent_symlinks_are_rejected_without_touching_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            real_parent = root / "real-parent"
            real_parent.mkdir()
            preserved = real_parent / "preserved"
            preserved.write_text("keep", encoding="utf-8")
            parent_link = root / "parent-link"
            parent_link.symlink_to(real_parent, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, "symlink"):
                syft.Installer(parent_link / "new-output", system="Darwin", machine="arm64")
            output_link = root / "output-link"
            output_link.symlink_to(real_parent, target_is_directory=True)
            with self.assertRaises(FileExistsError):
                syft.Installer(output_link, system="Darwin", machine="arm64")
            self.assertEqual(preserved.read_text(encoding="utf-8"), "keep")

    def test_output_ancestor_swap_to_symlink_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            parent = root / "output-parent"
            parent.mkdir()
            target = root / "outside"
            target.mkdir()
            marker = target / "marker"
            marker.write_text("preserve", encoding="utf-8")
            moved = root / "output-parent-held"
            original_open = os.open
            swapped = False
            def swap_then_open(path, flags, *args, **kwargs):
                nonlocal swapped
                if path == "output-parent" and kwargs.get("dir_fd") is not None and not swapped:
                    parent.rename(moved)
                    parent.symlink_to(target, target_is_directory=True)
                    swapped = True
                return original_open(path, flags, *args, **kwargs)
            with mock.patch.object(syft.os, "open", side_effect=swap_then_open):
                with self.assertRaisesRegex(RuntimeError, "symlink"):
                    syft.Installer(parent / "new-output", system="Darwin", machine="arm64")
            self.assertTrue(swapped)
            self.assertEqual(marker.read_text(encoding="utf-8"), "preserve")

    def test_gzip_tar_metadata_and_decompressed_stream_are_bounded_before_tarfile(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            oversized = root / "oversized-metadata.tar.gz"
            metadata = b"x" * (syft.MAX_METADATA_BYTES + 1)
            pax = tarfile.TarInfo("PaxHeaders.0/syft")
            pax.type = tarfile.XHDTYPE
            pax.size = len(metadata)
            content = io.BytesIO()
            with tarfile.open(fileobj=content, mode="w") as tar:
                tar.addfile(pax, io.BytesIO(metadata))
            oversized.write_bytes(gzip.compress(content.getvalue()))
            with self.assertRaisesRegex(ValueError, "metadata header exceeds"):
                syft._scan_tar_gzip_bounds(oversized.read_bytes())

            regular = root / "large-expanded.tar.gz"
            entries = {"CHANGELOG.md": b"c" * 1024, "LICENSE": b"l", "README.md": b"r", "syft": b"s"}
            self._make_archive(regular, entries)
            with mock.patch.object(syft, "MAX_ARCHIVE_STREAM_BYTES", 2048):
                with self.assertRaisesRegex(ValueError, "decompressed stream limit"):
                    syft._scan_tar_gzip_bounds(regular.read_bytes())

    def test_child_environment_is_fixed_and_omits_credentials_proxy_and_ca_overrides(self):
        with tempfile.TemporaryDirectory() as temporary:
            installer = syft.Installer(Path(temporary).resolve() / "output", system="Darwin", machine="arm64")
            env = installer._minimal_env()
            self.assertEqual(env["PATH"], "/usr/bin:/bin")
            self.assertEqual(env["PIP_CONFIG_FILE"], os.devnull)
            for name in ("GH_TOKEN", "GITHUB_TOKEN", "PIP_INDEX_URL", "PIP_EXTRA_INDEX_URL", "PIP_CERT",
                         "HTTPS_PROXY", "HTTP_PROXY", "REQUESTS_CA_BUNDLE", "CURL_CA_BUNDLE", "SSL_CERT_FILE"):
                self.assertNotIn(name, env)

    def test_pip_configuration_audit_precedes_hash_locked_install(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / "output"
            installer = syft.Installer(output, system="Darwin", machine="arm64")
            installer._make_output()
            captured = []
            def execute(argv, **kwargs):
                captured.append((argv, kwargs["env"]))
                if "-m" in argv and "venv" in argv:
                    venv_python = Path(argv[-1]) / "bin" / "python"
                    venv_python.parent.mkdir(parents=True)
                    venv_python.symlink_to(sys.executable)
                if len(argv) > 1 and Path(argv[1]).name == "pip-config-audit.py":
                    return syft.run_bounded_command(argv, cwd=kwargs["cwd"], env=kwargs["env"],
                                                    timeout=kwargs["timeout"], log_path=kwargs["log_path"])
                syft.safe_write(kwargs["log_path"], b"fixture output\n")
                return 0, "", ""
            installer.execute_process = execute
            lock = root / "retained.lock"
            lock.write_bytes(b"locked requirements\n")
            installer._install_verifier(lock)
            audit_index = next(i for i, pair in enumerate(captured) if Path(pair[0][1]).name == "pip-config-audit.py")
            install_index = next(i for i, pair in enumerate(captured) if "install" in pair[0])
            self.assertLess(audit_index, install_index)
            install_argv = captured[install_index][0]
            self.assertIn("--isolated", install_argv)
            self.assertIn("--require-hashes", install_argv)
            self.assertEqual(install_argv[-1], str(lock))
            audit_source = (output / "pip-config-audit.py").read_text(encoding="utf-8")
            self.assertIn("config.load()", audit_source)
            self.assertIn("config.get_values_in_config", audit_source)
            self.assertIn("config-canary.invalid", audit_source)
            for _argv, env in captured:
                self.assertEqual(env["PATH"], "/usr/bin:/bin")
                self.assertEqual(env["PIP_CONFIG_FILE"], os.devnull)

    def test_download_write_race_with_symlink_does_not_overwrite_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / "output"
            sentinel = root / "sentinel"
            sentinel.write_bytes(b"preserve")
            def race(_url, destination, _limit):
                destination.symlink_to(sentinel)
            installer = syft.Installer(output, fetch=race, system="Darwin", machine="arm64")
            installer._make_output()
            with self.assertRaisesRegex(RuntimeError, "regular non-symlink file"):
                installer._download("fixture", syft.CHECKSUM_URL, "file", 16)
            self.assertEqual(sentinel.read_bytes(), b"preserve")

    def test_failed_cleanup_does_not_follow_replaced_output_symlink(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / "output"
            external = root / "external"
            external.mkdir()
            sentinel = external / "sentinel"
            sentinel.write_text("preserve", encoding="utf-8")
            installer = syft.Installer(output, system="Darwin", machine="arm64")
            installer._make_output()
            moved = root / "output-held"
            output.rename(moved)
            output.symlink_to(external, target_is_directory=True)
            self.assertFalse(syft._remove_owned_directory(output, installer.output_identity))
            self.assertTrue(output.is_symlink())
            self.assertTrue((moved / "logs").is_dir())
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")

    def test_existing_output_is_never_removed(self):
        with tempfile.TemporaryDirectory() as temporary:
            existing = Path(temporary).resolve() / "existing"
            existing.mkdir()
            sentinel = existing / "keep"
            sentinel.write_text("preserve", encoding="utf-8")
            with self.assertRaises(FileExistsError):
                syft.Installer(existing, system="Darwin", machine="arm64")
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")

    def test_sigstore_github_command_pins_identity_sha_repository_ref_and_issuer_policy(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            checksums = root / "checksums"
            bundle = root / "bundle"
            checksums.write_bytes(b"signed checksums")
            bundle.write_bytes(b"signed bundle")
            captured = []
            def execute(argv, **kwargs):
                captured.append(argv)
                syft.safe_write(kwargs["log_path"], b"verified fixture only\n")
                return 0, "", ""
            installer = syft.Installer(root / "out", execute=execute, system="Darwin", machine="arm64")
            (root / "out").mkdir()
            (root / "out" / "logs").mkdir()
            installer.output_identity = (os.stat(root / "out").st_dev, os.stat(root / "out").st_ino)
            installer._verify_checksums(Path("/venv/python"), checksums, bundle)
            args = captured[0]
            self.assertIn("verify", args)
            self.assertIn("github", args)
            for value in ("--cert-identity", syft.CERT_IDENTITY, "--sha", syft.RELEASE_COMMIT,
                          "--repository", syft.REPOSITORY, "--ref", syft.WORKFLOW_REF, str(checksums)):
                self.assertIn(value, args)
            # Sigstore 4.5.0 verify-github hardcodes this issuer; it has no issuer flag.
            self.assertEqual(syft.OIDC_ISSUER, "https://token.actions.githubusercontent.com")
            self.assertNotIn("--cert-oidc-issuer", args)
            self.assertEqual(len(captured), 1)

    def test_injected_success_path_emits_receipt_but_is_not_crypto_qualification(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            asset = "syft_1.54.0_darwin_arm64.tar.gz"
            archive_path = root / "fixture.tar.gz"
            self._make_archive(archive_path, {"CHANGELOG.md": b"changelog", "LICENSE": b"license", "README.md": b"readme", "syft": b"fixture executable"})
            archive_bytes = archive_path.read_bytes()
            checksum_bytes = f"{hashlib.sha256(archive_bytes).hexdigest()}  {asset}\n".encode()
            bundle_bytes = b"injected-test-bundle"
            values = {
                syft.CHECKSUM_URL: checksum_bytes,
                syft.BUNDLE_URL: bundle_bytes,
                f"https://github.com/anchore/syft/releases/download/{syft.RELEASE_TAG}/{asset}": archive_bytes,
            }
            target = dict(syft.detect_target("Darwin", "arm64"))
            target["sha256"] = hashlib.sha256(archive_bytes).hexdigest()
            target["binary_sha256"] = hashlib.sha256(b"fixture executable").hexdigest()
            version = {"application": "syft", "version": syft.VERSION, "gitCommit": syft.RELEASE_COMMIT, "platform": "darwin/arm64"}
            calls = []
            order = []
            def fetch(url, path, limit):
                order.append(url)
                self.assertLessEqual(len(values[url]), limit)
                syft.safe_write(path, values[url])
            def execute(argv, **kwargs):
                calls.append(argv)
                log = kwargs["log_path"]
                if "-m" in argv and "venv" in argv:
                    venv = Path(argv[-1]) / "bin"
                    venv.mkdir(parents=True)
                    (venv / "python").write_text("test-only fake", encoding="utf-8")
                syft.safe_write(log, b"injected process output\n")
                if "--extract-internal" in argv:
                    extractor = object.__new__(syft.Installer)
                    extractor.deadline = time.monotonic() + syft.TOTAL_TIMEOUT
                    _binary, extracted_archive = extractor._safe_extract(Path(argv[-3]), Path(argv[-2]), argv[-1])
                    return 0, json.dumps({"binary": "syft", "archive": extracted_archive}), ""
                if argv[-3:] == ["version", "-o", "json"]:
                    return 0, json.dumps(version), ""
                return 0, "", ""
            def install_verifier(_lock_path):
                order.append("install-verifier")
                return Path("/fake/verifier/python")
            def verify_checksums(_installer, _verifier, _checksums, _bundle):
                order.append("verify-checksum-signature")
            parse_checksum = syft.Installer._parse_checksum
            def parse_after_verification(data, selected_asset):
                order.append("parse-authenticated-checksum-row")
                return parse_checksum(data, selected_asset)
            retained_lock = (syft.LOCK_DIR / "syft-darwin-verifier.lock").read_bytes()
            with mock.patch.object(syft, "CHECKSUM_SHA256", hashlib.sha256(checksum_bytes).hexdigest()), \
                 mock.patch.object(syft, "BUNDLE_SHA256", hashlib.sha256(bundle_bytes).hexdigest()), \
                 mock.patch.object(syft, "validate_lock", return_value=(Path("fixture.lock"), retained_lock)), \
                 mock.patch.object(syft.Installer, "_install_verifier", side_effect=install_verifier), \
                 mock.patch.object(syft.Installer, "_verify_checksums", autospec=True, side_effect=verify_checksums) as verify, \
                 mock.patch.object(syft.Installer, "_parse_checksum", side_effect=parse_after_verification):
                installer = syft.Installer(root / "output", execute=execute, fetch=fetch, system="Darwin", machine="arm64")
                installer.target = target
                binary = installer.run()
            self.assertEqual(binary.read_bytes(), b"fixture executable")
            self.assertEqual(verify.call_count, 1)
            self.assertEqual((root / "output/verifier.lock").read_bytes(), retained_lock)
            self.assertEqual(order, [syft.CHECKSUM_URL, syft.BUNDLE_URL, "install-verifier",
                                     "verify-checksum-signature", "parse-authenticated-checksum-row",
                                     f"https://github.com/anchore/syft/releases/download/{syft.RELEASE_TAG}/{asset}"])
            receipt = json.loads((root / "output/evidence/receipt.json").read_text(encoding="utf-8"))
            self.assertEqual(receipt["result"], "pass")
            self.assertEqual(receipt["qualification_limit"], "Native authenticated installation and version probe only; no package scan, release, or publication is represented.")
            self.assertEqual(len(receipt["installer_source_sha256"]), 64)
            self.assertEqual(len(receipt["python_toolchain"]["executable_sha256"]), 64)
            self.assertNotIn("crypto_verified", receipt)

    def test_authenticated_row_mismatch_stops_before_archive_download_and_cleans_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            checksum = b"a" * 64 + b"  syft_1.54.0_darwin_arm64.tar.gz\n"
            bundle = b"bundle"
            values = {syft.CHECKSUM_URL: checksum, syft.BUNDLE_URL: bundle}
            fetches = []
            def fetch(url, path, limit):
                fetches.append(url)
                syft.safe_write(path, values[url])
            def execute(argv, **kwargs):
                syft.safe_write(kwargs["log_path"], b"ok\n")
                return 0, "", ""
            retained_lock = (syft.LOCK_DIR / "syft-darwin-verifier.lock").read_bytes()
            with mock.patch.object(syft, "CHECKSUM_SHA256", hashlib.sha256(checksum).hexdigest()), \
                 mock.patch.object(syft, "BUNDLE_SHA256", hashlib.sha256(bundle).hexdigest()), \
                 mock.patch.object(syft, "validate_lock", return_value=(Path("fixture.lock"), retained_lock)), \
                 mock.patch.object(syft.Installer, "_install_verifier", return_value=Path("/fake/python")), \
                 mock.patch.object(syft.Installer, "_verify_checksums", autospec=True):
                installer = syft.Installer(root / "out", execute=execute, fetch=fetch, system="Darwin", machine="arm64")
                with self.assertRaisesRegex(RuntimeError, "differs from the retained"):
                    installer.run()
            self.assertEqual(fetches, [syft.CHECKSUM_URL, syft.BUNDLE_URL])
            self.assertFalse((root / "out").exists())

    def test_checksum_or_bundle_digest_mismatch_stops_before_verifier_install(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            checksum = b"untrusted checksum text"
            bundle = b"untrusted bundle"
            values = {syft.CHECKSUM_URL: checksum, syft.BUNDLE_URL: bundle}
            fetches = []
            def fetch(url, path, _limit):
                fetches.append(url)
                syft.safe_write(path, values[url])
            retained_lock = (syft.LOCK_DIR / "syft-darwin-verifier.lock").read_bytes()
            with mock.patch.object(syft, "CHECKSUM_SHA256", "0" * 64), \
                 mock.patch.object(syft, "BUNDLE_SHA256", "1" * 64), \
                 mock.patch.object(syft, "validate_lock", return_value=(Path("fixture.lock"), retained_lock)), \
                 mock.patch.object(syft.Installer, "_install_verifier") as install:
                installer = syft.Installer(root / "out", fetch=fetch, system="Darwin", machine="arm64")
                with self.assertRaisesRegex(RuntimeError, "retained checksum or bundle digest mismatch"):
                    installer.run()
            install.assert_not_called()
            self.assertEqual(fetches, [syft.CHECKSUM_URL, syft.BUNDLE_URL])
            self.assertFalse((root / "out").exists())

    def test_nonzero_sigstore_result_stops_before_checksum_parse_or_archive_fetch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            asset = "syft_1.54.0_darwin_arm64.tar.gz"
            digest = syft.TARGETS[("Darwin", "arm64")]["sha256"]
            checksum = f"{digest}  {asset}\n".encode()
            bundle = b"injected bundle"
            values = {syft.CHECKSUM_URL: checksum, syft.BUNDLE_URL: bundle}
            fetches = []
            def fetch(url, path, _limit):
                fetches.append(url)
                syft.safe_write(path, values[url])
            commands = []
            def execute(argv, **kwargs):
                commands.append(argv)
                syft.safe_write(kwargs["log_path"], b"injected verifier rejection\n")
                return 1, "", "rejected"
            retained_lock = (syft.LOCK_DIR / "syft-darwin-verifier.lock").read_bytes()
            with mock.patch.object(syft, "CHECKSUM_SHA256", hashlib.sha256(checksum).hexdigest()), \
                 mock.patch.object(syft, "BUNDLE_SHA256", hashlib.sha256(bundle).hexdigest()), \
                 mock.patch.object(syft, "validate_lock", return_value=(Path("fixture.lock"), retained_lock)), \
                 mock.patch.object(syft.Installer, "_install_verifier", return_value=Path("/fake/verifier/python")), \
                 mock.patch.object(syft.Installer, "_parse_checksum") as parse:
                installer = syft.Installer(root / "out", execute=execute, fetch=fetch,
                                           system="Darwin", machine="arm64")
                with self.assertRaisesRegex(RuntimeError, "verify-signed-checksum-document failed"):
                    installer.run()
            parse.assert_not_called()
            self.assertEqual(fetches, [syft.CHECKSUM_URL, syft.BUNDLE_URL])
            self.assertIn("verify", commands[-1])
            self.assertIn("github", commands[-1])
            self.assertFalse((root / "out").exists())

    def test_public_cli_has_no_cross_target_override(self):
        with self.assertRaises(SystemExit) as raised:
            syft.main(["--output-dir", "/does/not/matter", "--target", "linux-amd64"])
        self.assertEqual(raised.exception.code, 2)

    def test_total_deadline_and_combined_stdout_stderr_caps_kill_process_group(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            log = root / "timeout.log"
            program = "import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print('ready', flush=True); time.sleep(30)"
            started = time.monotonic()
            with self.assertRaises(subprocess.TimeoutExpired):
                syft.run_bounded_command([sys.executable, "-c", program], cwd=root, env=os.environ.copy(), timeout=0.25, log_path=log)
            self.assertLess(time.monotonic() - started, 4)
            self.assertTrue(log.exists())
            overflow = root / "overflow.log"
            program = "import sys; print('x'*10000); print('y'*10000, file=sys.stderr)"
            with self.assertRaisesRegex(RuntimeError, "output exceeded"):
                syft.run_bounded_command([sys.executable, "-c", program], cwd=root, env=os.environ.copy(), timeout=5, log_path=overflow, max_output_bytes=1024)
            self.assertEqual(overflow.stat().st_size, 1024)

    def test_parent_exit_with_child_holding_output_pipe_is_timed_out_and_killed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            marker = root / "child-pid"
            child = "import os,time,pathlib; pathlib.Path(os.environ['CHILD_PID']).write_text(str(os.getpid())); time.sleep(30)"
            program = "\n".join([
                "import os,pathlib,subprocess,sys,time",
                f"subprocess.Popen([sys.executable, '-c', {child!r}])",
                "deadline=time.monotonic()+6",
                "marker=pathlib.Path(os.environ['CHILD_PID'])",
                "while not marker.exists() and time.monotonic()<deadline:",
                "    time.sleep(0.01)",
                "assert marker.exists(), 'child readiness marker missing'",
                "print('parent-exited', flush=True)",
            ])
            log = root / "child-writer.log"
            env = os.environ.copy()
            env["CHILD_PID"] = str(marker)
            started = time.monotonic()
            with self.assertRaises(subprocess.TimeoutExpired):
                syft.run_bounded_command([sys.executable, "-c", program], cwd=root, env=env,
                                         timeout=3, log_path=log)
            self.assertLess(time.monotonic() - started, 5)
            self.assertTrue(marker.exists())
            self.assertIn(b"parent-exited", log.read_bytes())
            child_pid = int(marker.read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(child_pid, 0)
            self.assertLessEqual(log.stat().st_size, syft.MAX_LOG_BYTES)

    def test_keyboard_interrupt_stops_process_group_and_preserves_interrupt(self):
        import selectors
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            marker = root / "pid"
            program = "import os,signal,time,pathlib; signal.signal(signal.SIGTERM,signal.SIG_IGN); pathlib.Path(os.environ['PID_FILE']).write_text(str(os.getpid())); time.sleep(30)"
            real_select = selectors.DefaultSelector.select
            def interrupt_after_child(selector, timeout=None):
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline and not marker.exists():
                    time.sleep(0.01)
                self.assertTrue(marker.exists())
                raise KeyboardInterrupt
            log = root / "interrupt.log"
            env = os.environ.copy()
            env["PID_FILE"] = str(marker)
            with mock.patch("selectors.DefaultSelector.select", new=interrupt_after_child):
                with self.assertRaises(KeyboardInterrupt):
                    syft.run_bounded_command([sys.executable, "-c", program], cwd=root, env=env, timeout=5, log_path=log)
            pid = int(marker.read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)


if __name__ == "__main__":
    unittest.main()
