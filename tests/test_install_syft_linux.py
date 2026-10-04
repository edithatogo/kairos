from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest

_INSTALLER_PATH = Path(__file__).resolve().parents[1] / "packaging" / "scripts" / "install_syft_linux.py"
_INSTALLER_SPEC = importlib.util.spec_from_file_location("install_syft_linux", _INSTALLER_PATH)
assert _INSTALLER_SPEC is not None and _INSTALLER_SPEC.loader is not None
installer = importlib.util.module_from_spec(_INSTALLER_SPEC)
sys.modules[_INSTALLER_SPEC.name] = installer
_INSTALLER_SPEC.loader.exec_module(installer)


class FakeResponse:
    def __init__(self, url: str, body: bytes, *, headers: dict[str, str] | None = None):
        self._url = url
        self._body = io.BytesIO(body)
        self.headers = headers or {"Content-Length": str(len(body)), "Content-Encoding": "identity"}

    def geturl(self):
        return self._url

    def read(self, size=-1):
        return self._body.read(size)

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self._body.close()


class Fixture:
    def __init__(self, root: Path):
        self.root = root
        self.output_parent = root / "outputs"
        self.evidence_parent = root / "evidence"
        self.output_parent.mkdir()
        self.evidence_parent.mkdir()
        self.call_log = root / "calls.jsonl"
        self.syft_bytes = (
            "#!/usr/bin/env python3\n"
            "import json,sys\n"
            f"open({str(self.call_log)!r},'a').write('syft\\n')\n"
            "print(json.dumps({'application':'syft','version':'vtest',"
            "'gitCommit':'0123456789abcdef0123456789abcdef01234567',"
            "'platform':'linux/amd64'}))\n"
        ).encode()
        self.cosign_bytes = (
            "#!/usr/bin/env python3\n"
            "import json,sys\n"
            f"open({str(self.call_log)!r},'a').write('cosign:'+sys.argv[1]+'\\n')\n"
            "if sys.argv[1:] == ['version','--json']:\n"
            " print(json.dumps({'gitVersion':'v3.1.3'})); raise SystemExit(0)\n"
            "if sys.argv[1] == 'verify-blob':\n"
            " expected=['--certificate-identity',"
            f"{installer.WORKFLOW_IDENTITY!r},"
            "'--certificate-oidc-issuer',"
            f"{installer.OIDC_ISSUER!r},"
            "'--certificate-github-workflow-repository','anchore/syft',"
            "'--certificate-github-workflow-sha','0123456789abcdef0123456789abcdef01234567',"
            "'--certificate-github-workflow-ref','refs/heads/main']\n"
            " assert all(x in sys.argv for x in expected), sys.argv\n"
            " raise SystemExit(0)\n"
            "raise SystemExit(9)\n"
        ).encode()
        self.cosign = root / "cosign"
        self.cosign.write_bytes(self.cosign_bytes)
        self.cosign.chmod(0o700)
        self.checksum_name = "checksums.txt"
        self.bundle_name = "checksums.txt.sigstore.json"
        self.archive_name = "syft_test_linux_amd64.tar.gz"
        self.archive = self._archive()
        self.archive_sha = hashlib.sha256(self.archive).hexdigest()
        self.checksums = f"{self.archive_sha}  {self.archive_name}\n".encode()
        self.bundle = b'{"fixture":"sigstore bundle"}\n'
        self.urls = {
            "https://github.com/anchore/syft/releases/download/vtest/checksums.txt": self.checksums,
            "https://github.com/anchore/syft/releases/download/vtest/checksums.txt.sigstore.json": self.bundle,
            "https://github.com/anchore/syft/releases/download/vtest/syft_test_linux_amd64.tar.gz": self.archive,
        }
        self.pins = installer.ReleasePins(
            version="vtest",
            git_commit="0123456789abcdef0123456789abcdef01234567",
            archive_name=self.archive_name,
            archive_url="https://github.com/anchore/syft/releases/download/vtest/" + self.archive_name,
            archive_size=len(self.archive),
            archive_sha256=self.archive_sha,
            checksum_name=self.checksum_name,
            checksum_url="https://github.com/anchore/syft/releases/download/vtest/" + self.checksum_name,
            checksum_size=len(self.checksums),
            checksum_sha256=hashlib.sha256(self.checksums).hexdigest(),
            bundle_name=self.bundle_name,
            bundle_url="https://github.com/anchore/syft/releases/download/vtest/" + self.bundle_name,
            bundle_size=len(self.bundle),
            bundle_sha256=hashlib.sha256(self.bundle).hexdigest(),
            binary_sha256=hashlib.sha256(self.syft_bytes).hexdigest(),
            members=(
                installer.MemberPin("CHANGELOG.md", 3),
                installer.MemberPin("LICENSE", 4),
                installer.MemberPin("README.md", 5),
                installer.MemberPin("syft", len(self.syft_bytes), hashlib.sha256(self.syft_bytes).hexdigest()),
            ),
        )
        self.requests: list[tuple[str, int]] = []

    def _archive(self) -> bytes:
        items = (("CHANGELOG.md", b"log"), ("LICENSE", b"lic!"), ("README.md", b"read!"), ("syft", self.syft_bytes))
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode="w:gz") as archive:
            for name, body in items:
                member = tarfile.TarInfo(name)
                member.size = len(body)
                member.mode = 0o755 if name == "syft" else 0o644
                archive.addfile(member, io.BytesIO(body))
        return output.getvalue()

    def opener(self, *, replacements=None, fail_url=None, headers=None):
        bodies = dict(self.urls)
        bodies.update(replacements or {})

        def open_url(request, timeout):
            url = request.full_url
            self.requests.append((url, timeout))
            if url == fail_url:
                raise TimeoutError("offline fixture timeout")
            if url not in bodies:
                raise AssertionError(f"unexpected network request: {url}")
            return FakeResponse(url, bodies[url], headers=headers or {"Content-Length": str(len(bodies[url])), "Content-Encoding": "identity"})

        return open_url

    def run(self, *, output_name="syft", evidence_name="run", **kwargs):
        check_platform = kwargs.pop("check_platform", False)
        return installer.install(
            self.output_parent / output_name,
            self.evidence_parent / evidence_name,
            str(self.cosign),
            hashlib.sha256(self.cosign_bytes).hexdigest(),
            pins=self.pins,
            open_url=kwargs.pop("open_url", self.opener()),
            check_platform=check_platform,
            **kwargs,
        )


class SyftInstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.fixture = Fixture(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_success_authenticates_extracts_checks_and_publishes_once(self):
        fixture = self.fixture
        output = fixture.run()
        self.assertEqual(output.read_bytes(), fixture.syft_bytes)
        self.assertEqual(hashlib.sha256(output.read_bytes()).hexdigest(), fixture.pins.binary_sha256)
        receipt = json.loads((fixture.evidence_parent / "run" / "receipt.json").read_text())
        self.assertEqual(receipt["status"], "verified")
        self.assertEqual([stage["stage"] for stage in receipt["stages"]], [
            "download-checksums", "download-signature-bundle", "cosign-version",
            "cosign-verify-blob", "release-checksum-entry", "download-linux-archive",
            "extract-and-hash-binary", "syft-version",
        ])
        self.assertEqual([url for url, _ in fixture.requests], list(fixture.urls))
        self.assertTrue(all(timeout == installer.READ_TIMEOUT_SECONDS for _, timeout in fixture.requests))
        self.assertEqual(fixture.call_log.read_text().splitlines(), ["cosign:version", "cosign:verify-blob", "syft"])

    def test_checksum_parser_requires_one_exact_filename_entry(self):
        digest = "a" * 64
        installer._checksum_line(f"{digest}  archive.tgz\n".encode(), "archive.tgz", digest)
        for bad in (
            f"{digest} *archive.tgz\n".encode(),
            f"{digest}  archive.tgz\n{digest}  archive.tgz\n".encode(),
            f"{'b' * 64}  archive.tgz\n".encode(),
            b"\xff\n",
        ):
            with self.subTest(bad=bad), self.assertRaises(installer.InstallError):
                installer._checksum_line(bad, "archive.tgz", digest)

    def test_wrong_signature_bundle_hash_stops_before_cosign_and_archive(self):
        fixture = self.fixture
        altered = b"x" * len(fixture.bundle)
        opener = fixture.opener(replacements={fixture.pins.bundle_url: altered})
        with self.assertRaisesRegex(installer.InstallError, "SHA-256"):
            fixture.run(open_url=opener)
        self.assertEqual(len(fixture.requests), 2)
        self.assertFalse(fixture.call_log.exists())
        receipt = json.loads((fixture.evidence_parent / "run" / "receipt.json").read_text())
        self.assertEqual(receipt["status"], "failed")
        self.assertFalse((fixture.output_parent / "syft").exists())

    def test_signature_identity_arguments_and_cosign_hash_are_enforced(self):
        fixture = self.fixture
        changed = fixture.cosign.with_name("changed-cosign")
        changed.write_bytes(fixture.cosign_bytes + b"#changed\n")
        changed.chmod(0o700)
        with self.assertRaisesRegex(installer.InstallError, "SHA-256"):
            installer.install(fixture.output_parent / "syft", fixture.evidence_parent / "bad-hash",
                              str(changed), hashlib.sha256(fixture.cosign_bytes).hexdigest(),
                              pins=fixture.pins, open_url=fixture.opener(), check_platform=False)
        self.assertEqual(fixture.requests, [])
        self.assertFalse((fixture.evidence_parent / "bad-hash").exists())

        linked = fixture.root / "cosign-link"
        linked.symlink_to(fixture.cosign)
        with self.assertRaisesRegex(installer.InstallError, "symlink"):
            installer.install(fixture.output_parent / "syft", fixture.evidence_parent / "symlink-cosign",
                              str(linked), hashlib.sha256(fixture.cosign_bytes).hexdigest(),
                              pins=fixture.pins, open_url=fixture.opener(), check_platform=False)
        self.assertEqual(fixture.requests, [])

        # A correctly hashed executable still must satisfy the pinned identity arguments.
        script = fixture.cosign.read_text().replace("anchore/syft", "attacker/syft")
        fixture.cosign.write_text(script)
        fixture.cosign.chmod(0o700)
        digest = hashlib.sha256(fixture.cosign.read_bytes()).hexdigest()
        with self.assertRaisesRegex(installer.InstallError, "exited"):
            installer.install(fixture.output_parent / "syft", fixture.evidence_parent / "bad-signature",
                              str(fixture.cosign), digest, pins=fixture.pins,
                              open_url=fixture.opener(), check_platform=False)
        self.assertFalse((fixture.output_parent / "syft").exists())
        self.assertTrue((fixture.evidence_parent / "bad-signature" / "cosign-verify-blob.stderr.log").exists())

    def test_cosign_version_and_executable_replacement_are_rejected(self):
        fixture = self.fixture
        original = fixture.cosign_bytes
        fixture.cosign.write_text("#!/usr/bin/env python3\nprint('{\"gitVersion\":\"v0.0.0\"}')\n")
        fixture.cosign.chmod(0o700)
        digest = hashlib.sha256(fixture.cosign.read_bytes()).hexdigest()
        with self.assertRaisesRegex(installer.InstallError, "version differs"):
            installer.install(fixture.output_parent / "syft", fixture.evidence_parent / "old-version",
                              str(fixture.cosign), digest, pins=fixture.pins,
                              open_url=fixture.opener(), check_platform=False)
        self.assertFalse((fixture.evidence_parent / "old-version" / "cosign-verify-blob.stdout.log").exists())
        fixture.cosign.write_bytes(original)
        fixture.cosign.chmod(0o700)
        self.assertEqual(hashlib.sha256(fixture.cosign.read_bytes()).hexdigest(), hashlib.sha256(original).hexdigest())

        replacement = (
            "#!/usr/bin/env python3\n"
            "import json\n"
            "with open(__file__, 'ab') as f: f.write(b'# replaced after exec\\n')\n"
            "print(json.dumps({'gitVersion':'v3.1.3'}))\n"
        )
        fixture.cosign.write_text(replacement)
        fixture.cosign.chmod(0o700)
        replacement_hash = hashlib.sha256(fixture.cosign.read_bytes()).hexdigest()
        with self.assertRaisesRegex(installer.InstallError, "identity changed"):
            installer.install(fixture.output_parent / "syft", fixture.evidence_parent / "replaced-cosign",
                              str(fixture.cosign), replacement_hash, pins=fixture.pins,
                              open_url=fixture.opener(), check_platform=False)
        evidence = fixture.evidence_parent / "replaced-cosign"
        self.assertTrue((evidence / "cosign-version.stdout.log").is_file())
        self.assertFalse((evidence / "cosign-verify-blob.stdout.log").exists())
        self.assertFalse((fixture.output_parent / "syft").exists())

    def test_cosign_is_rehashed_immediately_before_first_invocation(self):
        fixture = self.fixture
        base_open = fixture.opener()
        calls = 0

        def replace_after_download(request, timeout):
            nonlocal calls
            calls += 1
            response = base_open(request, timeout)
            if calls == 2:
                fixture.cosign.write_bytes(fixture.cosign_bytes + b"# substituted after download\n")
                fixture.cosign.chmod(0o700)
            return response

        with self.assertRaisesRegex(installer.InstallError, "identity changed"):
            fixture.run(evidence_name="replaced-before-version", open_url=replace_after_download)
        evidence = fixture.evidence_parent / "replaced-before-version"
        self.assertFalse((evidence / "cosign-version.stdout.log").exists())
        self.assertFalse(fixture.call_log.exists())
        self.assertFalse((fixture.output_parent / "syft").exists())

    def test_download_timeout_and_monotonic_deadline_remove_partial_asset(self):
        fixture = self.fixture
        url = fixture.pins.checksum_url
        destination = fixture.root / "partial"
        ticks = iter((0.0, 0.0, 2.0, 2.0))
        def slow_open(request, timeout):
            return FakeResponse(request.full_url, b"x" * fixture.pins.checksum_size)
        with self.assertRaisesRegex(installer.InstallError, "deadline"):
            installer._download(url, destination, fixture.pins.checksum_size,
                                fixture.pins.checksum_sha256, open_url=slow_open,
                                monotonic=lambda: next(ticks), deadline_seconds=1)
        self.assertFalse(destination.exists())

        with self.assertRaises(TimeoutError):
            installer._download(url, destination, fixture.pins.checksum_size,
                                fixture.pins.checksum_sha256,
                                open_url=lambda *_args, **_kwargs: (_ for _ in ()).throw(TimeoutError("read timeout")))
        self.assertFalse(destination.exists())

    def test_download_rejects_url_redirect_encoding_length_and_overflow(self):
        fixture = self.fixture
        url = fixture.pins.checksum_url
        body = fixture.checksums
        cases = [
            (lambda request: FakeResponse("http://github.com/x", body), "response URL"),
            (lambda request: FakeResponse(url, body, headers={"Content-Encoding": "gzip"}), "encoding"),
            (lambda request: FakeResponse(url, body, headers={"Content-Length": "1"}), "Content-Length"),
            (lambda request: FakeResponse(url, body + b"x", headers={"Content-Length": str(len(body))}), "exceeds pinned size"),
        ]
        for index, (response, message) in enumerate(cases):
            dest = fixture.root / f"bad-download-{index}"
            with self.subTest(message=message), self.assertRaisesRegex(installer.InstallError, message):
                installer._download(url, dest, len(body), hashlib.sha256(body).hexdigest(),
                                    open_url=lambda request, timeout, response=response: response(request))
            self.assertFalse(dest.exists())

        with self.assertRaisesRegex(installer.InstallError, "allowlist"):
            installer._download("https://evil.invalid/a", fixture.root / "bad-url", 1, "a" * 64)

    def test_archive_extraction_rejects_traversal_duplicates_links_and_missing_files(self):
        fixture = self.fixture

        def make_archive(entries):
            stream = io.BytesIO()
            with tarfile.open(fileobj=stream, mode="w:gz") as tar:
                for name, data, kind, *declared in entries:
                    item = tarfile.TarInfo(name)
                    item.size = declared[0] if declared else len(data)
                    if kind == "symlink":
                        item.type = tarfile.SYMTYPE
                        item.linkname = "syft"
                        item.size = 0
                        tar.addfile(item)
                    elif kind == "hardlink":
                        item.type = tarfile.LNKTYPE
                        item.linkname = "LICENSE"
                        item.size = 0
                        tar.addfile(item)
                    elif kind == "fifo":
                        item.type = tarfile.FIFOTYPE
                        item.size = 0
                        tar.addfile(item)
                    elif kind == "pax":
                        item.pax_headers = {"comment": "x" * 70_000}
                        item.size = len(data)
                        tar.addfile(item, io.BytesIO(data))
                    else:
                        tar.addfile(item, io.BytesIO(data))
            path = fixture.root / "custom.tgz"
            path.write_bytes(stream.getvalue())
            return path

        good = [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
                ("README.md", b"read!", "file"), ("syft", fixture.syft_bytes, "file")]
        for index, bad_entries in enumerate((
            [("../escape", b"x", "file"), *good],
            [*good, ("syft", fixture.syft_bytes, "file")],
            [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
             ("README.md", b"read!", "file"), ("syft", b"", "symlink")],
            [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
             ("README.md", b"read!", "file"), ("syft", b"", "hardlink")],
            [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
             ("README.md", b"read!", "file"), ("syft", b"", "fifo")],
            [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
             ("README.md", b"read!", "file"), ("syft", fixture.syft_bytes, "file", 1)],
            [("CHANGELOG.md", b"log", "file"), ("LICENSE", b"lic!", "file"),
             ("README.md", b"read!", "file"), ("syft", fixture.syft_bytes, "pax")],
            good[:-1],
        )):
            dest = fixture.root / f"extract-{index}"
            with self.subTest(index=index), self.assertRaises(installer.InstallError):
                installer._extract_archive(make_archive(bad_entries), dest, fixture.pins)
            self.assertFalse(dest.exists())
        broken = fixture.root / "truncated.tgz"
        broken.write_bytes(fixture.archive[:20])
        with self.assertRaises(installer.InstallError):
            installer._extract_archive(broken, fixture.root / "extract-truncated", fixture.pins)

    def test_output_must_be_fresh_safe_and_distinct_from_evidence(self):
        fixture = self.fixture
        output = fixture.output_parent / "existing"
        output.write_text("preserve")
        with self.assertRaisesRegex(installer.InstallError, "fresh"):
            fixture.run(output_name="existing", evidence_name="existing-output")
        self.assertEqual(output.read_text(), "preserve")

        symlink_parent = fixture.root / "linked-parent"
        symlink_parent.symlink_to(fixture.output_parent, target_is_directory=True)
        with self.assertRaisesRegex(installer.InstallError, "symlink"):
            installer.install(symlink_parent / "syft", fixture.evidence_parent / "linked", str(fixture.cosign),
                              hashlib.sha256(fixture.cosign_bytes).hexdigest(), pins=fixture.pins,
                              open_url=fixture.opener(), check_platform=False)
        self.assertFalse((fixture.evidence_parent / "linked").exists())

    def test_exclusive_publication_does_not_overwrite_racing_destination(self):
        fixture = self.fixture
        output = fixture.output_parent / "raced-syft"
        original_link = installer.os.link

        def race_link(source, destination):
            Path(destination).write_bytes(b"created by a concurrent writer")
            return original_link(source, destination)

        installer.os.link = race_link
        try:
            with self.assertRaises(FileExistsError):
                fixture.run(output_name="raced-syft", evidence_name="publication-race")
        finally:
            installer.os.link = original_link
        self.assertEqual(output.read_bytes(), b"created by a concurrent writer")
        receipt = json.loads((fixture.evidence_parent / "publication-race" / "receipt.json").read_text())
        self.assertEqual(receipt["status"], "failed")

    def test_linux_platform_is_required_before_network(self):
        fixture = self.fixture
        old_platform, old_machine = installer.sys.platform, installer.platform.machine
        try:
            installer.sys.platform = "darwin"
            installer.platform.machine = lambda: "arm64"
            with self.assertRaisesRegex(installer.InstallError, "Linux x86_64"):
                fixture.run(check_platform=True)
        finally:
            installer.sys.platform, installer.platform.machine = old_platform, old_machine
        self.assertEqual(fixture.requests, [])

    def test_child_timeout_and_output_limit_are_bounded_and_logs_retained(self):
        fixture = self.fixture
        evidence = fixture.evidence_parent / "child-checks"
        evidence.mkdir()
        env = {"PATH": os.environ["PATH"], "HOME": os.environ.get("HOME", ""), "TMPDIR": str(fixture.root)}
        script = fixture.root / "noisy-child"
        script.write_text("#!/usr/bin/env python3\nprint('x'*10000)\n")
        script.chmod(0o700)
        with self.assertRaisesRegex(installer.InstallError, "output limit"):
            installer._run_bounded_child(script, [], "overflow", evidence, env, output_limit=128)
        self.assertLessEqual((evidence / "overflow.stdout.log").stat().st_size, 128)
        self.assertTrue((evidence / "overflow.stderr.log").exists())
        overflow_receipt = json.loads((evidence / "overflow.command.json").read_text())
        self.assertEqual(overflow_receipt["disposition"], "output-limit")
        self.assertEqual(overflow_receipt["argv"], [str(script)])
        self.assertEqual(overflow_receipt["cwd"], os.getcwd())
        self.assertTrue(overflow_receipt["started_at_utc"] < overflow_receipt["ended_at_utc"])

        sleeper = fixture.root / "slow-child"
        sleeper.write_text("#!/usr/bin/env python3\nimport time\nprint('started',flush=True)\ntime.sleep(5)\n")
        sleeper.chmod(0o700)
        with self.assertRaisesRegex(installer.InstallError, "timeout"):
            installer._run_bounded_child(sleeper, [], "timeout", evidence, env, timeout_seconds=1)
        self.assertIn(b"started", (evidence / "timeout.stdout.log").read_bytes())
        timeout_receipt = json.loads((evidence / "timeout.command.json").read_text())
        self.assertEqual(timeout_receipt["disposition"], "timeout")
        self.assertIsInstance(timeout_receipt["exit_code"], int)

    def test_child_descendant_cannot_hold_capture_pipes_past_timeout(self):
        fixture = self.fixture
        evidence = fixture.evidence_parent / "descendant-check"
        evidence.mkdir()
        env = {"PATH": os.environ["PATH"], "HOME": os.environ.get("HOME", ""), "TMPDIR": str(fixture.root)}
        sentinel = fixture.root / "descendant-survived"
        script = fixture.root / "spawning-parent"
        script.write_text(
            "#!/usr/bin/env python3\n"
            "import subprocess,sys\n"
            f"subprocess.Popen([sys.executable,'-c',\"import time; time.sleep(1.7); open({str(sentinel)!r},'w').write('alive')\"])\n"
            "print('parent exited',flush=True)\n"
        )
        script.chmod(0o700)
        start = time.monotonic()
        with self.assertRaisesRegex(installer.InstallError, "timeout"):
            installer._run_bounded_child(script, [], "descendant", evidence, env, timeout_seconds=1)
        self.assertLess(time.monotonic() - start, 3)
        self.assertIn(b"parent exited", (evidence / "descendant.stdout.log").read_bytes())
        receipt = json.loads((evidence / "descendant.command.json").read_text())
        self.assertEqual(receipt["disposition"], "timeout")
        time.sleep(1.8)
        self.assertFalse(sentinel.exists())

    def test_child_launch_and_pipe_read_failures_write_command_receipts(self):
        fixture = self.fixture
        evidence = fixture.evidence_parent / "runner-errors"
        evidence.mkdir()
        env = {"PATH": os.environ["PATH"], "HOME": os.environ.get("HOME", ""), "TMPDIR": str(fixture.root)}
        missing = fixture.root / "does-not-exist"
        with self.assertRaisesRegex(installer.InstallError, "failed to launch"):
            installer._run_bounded_child(missing, [], "launch-failure", evidence, env)
        launch = json.loads((evidence / "launch-failure.command.json").read_text())
        self.assertEqual(launch["disposition"], "launch-error")
        self.assertIsNone(launch["exit_code"])
        self.assertTrue((evidence / "launch-failure.stdout.log").is_file())
        self.assertTrue((evidence / "launch-failure.stderr.log").is_file())

        script = fixture.root / "read-error-child"
        script.write_text("#!/usr/bin/env python3\nimport time\nprint('partial',flush=True)\ntime.sleep(5)\n")
        script.chmod(0o700)
        original_popen, original_read = installer.subprocess.Popen, installer.os.read

        def patch_after_launch(*args, **kwargs):
            child = original_popen(*args, **kwargs)
            installer.os.read = lambda *_args: (_ for _ in ()).throw(OSError("injected pipe read error"))
            return child

        try:
            with self.assertRaisesRegex(installer.InstallError, "output read failed"):
                installer._run_bounded_child(script, [], "read-failure", evidence, env,
                                              timeout_seconds=2, popen=patch_after_launch)
        finally:
            installer.os.read = original_read
        read_failure = json.loads((evidence / "read-failure.command.json").read_text())
        self.assertEqual(read_failure["disposition"], "read-error")
        self.assertTrue((evidence / "read-failure.stdout.log").is_file())
        self.assertTrue((evidence / "read-failure.stderr.log").is_file())

    def test_failed_kill_and_both_bounded_waits_retain_unreaped_receipt(self):
        fixture = self.fixture
        evidence = fixture.evidence_parent / "unreaped-child"
        evidence.mkdir()
        read_out, write_out = os.pipe()
        read_err, write_err = os.pipe()
        os.write(write_out, b"partial stdout")
        os.write(write_err, b"partial stderr")

        class UnreapedProcess:
            pid = 2_147_483_647

            def __init__(self):
                self.stdout = os.fdopen(read_out, "rb", buffering=0)
                self.stderr = os.fdopen(read_err, "rb", buffering=0)
                self.wait_timeouts = []
                self.kill_calls = 0

            def poll(self):
                return None

            def wait(self, timeout=None):
                self.wait_timeouts.append(timeout)
                if timeout is None:
                    raise AssertionError("runner attempted an unbounded wait")
                raise subprocess.TimeoutExpired(cmd="stuck-child", timeout=timeout)

            def kill(self):
                self.kill_calls += 1

        child = UnreapedProcess()
        old_limit, old_killpg = installer.CHILD_KILL_DRAIN_SECONDS, installer.os.killpg
        installer.CHILD_KILL_DRAIN_SECONDS = 0.05
        installer.os.killpg = lambda *_args: (_ for _ in ()).throw(ProcessLookupError())
        try:
            with self.assertRaisesRegex(installer.InstallError, "cleanup timed out"):
                installer._run_bounded_child(
                    fixture.root / "synthetic-child", [], "unreaped", evidence,
                    {"PATH": os.environ["PATH"]}, timeout_seconds=0.05,
                    popen=lambda *_args, **_kwargs: child,
                )
        finally:
            installer.CHILD_KILL_DRAIN_SECONDS = old_limit
            installer.os.killpg = old_killpg
            os.close(write_out)
            os.close(write_err)
        self.assertEqual(child.wait_timeouts, [0.05, 0.05])
        self.assertGreaterEqual(child.kill_calls, 1)
        self.assertEqual((evidence / "unreaped.stdout.log").read_bytes(), b"partial stdout")
        self.assertEqual((evidence / "unreaped.stderr.log").read_bytes(), b"partial stderr")
        receipt = json.loads((evidence / "unreaped.command.json").read_text())
        self.assertEqual(receipt["disposition"], "cleanup-timeout")
        self.assertIsNone(receipt["exit_code"])
        self.assertIn("unreaped after bounded cleanup wait", receipt["failure"])


if __name__ == "__main__":
    unittest.main()
