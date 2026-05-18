#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SHELL_SCRIPTS = [
    "hpc/slurm/submit-experiment.sh",
    "hpc/slurm/submit-sweep.sh",
    "hpc/slurm/resume.sh",
    "docker/entrypoint.sh",
    "cloud/aws/submit-experiment.sh",
    "cloud/gcp/submit-experiment.sh",
]


def require(path: str, contains: list[str] | None = None) -> None:
    target = ROOT / path
    if not target.exists():
        raise AssertionError(f"missing {path}")
    text = target.read_text(encoding="utf-8")
    for needle in contains or []:
        if needle not in text:
            raise AssertionError(f"{path} missing {needle!r}")


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_json(path: str) -> dict:
    return json.loads(read(path))


def run_json(command: list[str]) -> dict:
    completed = subprocess.run(
        command,
        check=True,
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    return json.loads(completed.stdout)


def _strip_shell_comment(line: str) -> str:
    in_single = False
    in_double = False
    escaped = False
    for index, char in enumerate(line):
        if escaped:
            escaped = False
            continue
        if char == "\\" and not in_single:
            escaped = True
            continue
        if char == "'" and not in_double:
            in_single = not in_single
            continue
        if char == '"' and not in_single:
            in_double = not in_double
            continue
        if char == "#" and not in_single and not in_double:
            prefix = line[:index]
            if not prefix or prefix[-1].isspace():
                return prefix
    return line


def _assert_balanced_shell_tokens(path: str, text: str) -> None:
    try:
        shlex.split(text, posix=True)
    except ValueError as exc:
        raise AssertionError(f"{path} has unbalanced shell quoting: {exc}") from exc


def _shell_text_without_heredoc_bodies(path: str, lines: list[str]) -> tuple[str, list[tuple[str, int, str]]]:
    opener = re.compile(r"<<-?\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)['\"]?")
    stripped_lines: list[str] = []
    heredocs: list[tuple[str, int, str]] = []
    pending: tuple[str, int, bool, list[str]] | None = None
    for line_number, line in enumerate(lines, start=1):
        if pending:
            marker, start_line, allow_tabs, body = pending
            candidate = line.lstrip("\t") if allow_tabs else line
            if candidate == marker:
                heredocs.append((marker, start_line, "\n".join(body)))
                stripped_lines.append(line)
                pending = None
            else:
                body.append(line)
            continue
        stripped_lines.append(line)
        stripped = _strip_shell_comment(line)
        match = opener.search(stripped)
        if match:
            token = stripped[match.start() : match.end()]
            pending = (match.group(1), line_number, "<<-" in token, [])
    if pending:
        marker, line_number, _, _ = pending
        raise AssertionError(f"{path} has unterminated heredoc {marker!r} opened on line {line_number}")
    return "\n".join(stripped_lines), heredocs


def _assert_block_balance(path: str, text: str) -> None:
    lexer = shlex.shlex(text, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    lexer.commenters = "#"
    try:
        tokens = list(lexer)
    except ValueError as exc:
        raise AssertionError(f"{path} has unbalanced shell quoting: {exc}") from exc
    stack: list[tuple[str, str]] = []
    expected = {
        "if": "fi",
        "for": "done",
        "while": "done",
        "until": "done",
        "select": "done",
        "case": "esac",
    }
    closers = {"fi", "done", "esac"}
    for token in tokens:
        if token in expected:
            stack.append((token, expected[token]))
        elif token in closers:
            if not stack:
                raise AssertionError(f"{path} has unmatched shell block closer {token!r}")
            opener, closer = stack.pop()
            if token != closer:
                raise AssertionError(f"{path} closes {opener!r} with {token!r}; expected {closer!r}")
    if stack:
        opener, closer = stack[-1]
        raise AssertionError(f"{path} has unclosed shell block {opener!r}; expected {closer!r}")


def fallback_static_shell_validation(reason: str) -> None:
    print(
        f"warning: running limited static shell validation because bash -n is unavailable: {reason}",
        file=sys.stderr,
    )
    print(
        "warning: fallback checks shebangs, line endings, quoting, heredoc closure, and common block balance only; it is not equivalent to bash -n",
        file=sys.stderr,
    )
    for path in SHELL_SCRIPTS:
        text = read(path)
        lines = text.splitlines()
        assert lines, f"{path} is empty"
        assert lines[0] == "#!/usr/bin/env bash", f"{path} missing bash shebang"
        assert "\r" not in text, f"{path} contains CR line endings"
        shell_text, heredocs = _shell_text_without_heredoc_bodies(path, lines)
        _assert_balanced_shell_tokens(path, shell_text)
        _assert_block_balance(path, "\n".join(_strip_shell_comment(line) for line in shell_text.splitlines()))
        for marker, line_number, body in heredocs:
            if marker.startswith("SLURM"):
                generated_path = f"{path} heredoc {marker!r} opened on line {line_number}"
                _assert_balanced_shell_tokens(generated_path, body)
                _assert_block_balance(
                    generated_path,
                    "\n".join(_strip_shell_comment(line) for line in body.splitlines()),
                )


def validate_docker_surface() -> None:
    dockerfile = read("docker/Dockerfile")
    entrypoint = read("docker/entrypoint.sh")
    bake = read("docker/docker-bake.hcl")

    assert "FROM rust:" in dockerfile
    assert "FROM alpine:" in dockerfile
    assert "COPY --from=builder" in dockerfile
    assert "ENTRYPOINT" in dockerfile
    assert "USER kairo" in dockerfile
    assert "platforms = [\"linux/amd64\", \"linux/arm64\"]" in bake
    assert "trap on_term TERM INT" in entrypoint
    assert "checkpoint-manifest.json" in entrypoint
    assert ".tmp" in entrypoint and "mv \"$tmp_file\" \"$checkpoint_manifest\"" in entrypoint
    assert "resume --checkpoint" in entrypoint


def validate_cloud_manifests() -> None:
    aws_job = read("cloud/aws/batch-job-definition.yaml")
    aws_array = read("cloud/aws/batch-array-template.yaml")
    assert "jobDefinitionName: kairo-ecs-cli" in aws_job
    assert "type: container" in aws_job
    assert "platformCapabilities:" in aws_job and "FARGATE" in aws_job
    assert "resourceRequirements:" in aws_job
    assert "KAIRO_OUTPUT_URI" in aws_job
    assert "KAIRO_CHECKPOINT_DIR" in aws_job
    assert "arrayProperties:" in aws_array
    assert "${AWS_BATCH_JOB_ARRAY_INDEX}" in aws_array

    gcp_job = read_json("cloud/gcp/batch-job.json")
    gcp_array = read_json("cloud/gcp/batch-array.json")
    task_group = gcp_job["taskGroups"][0]
    runnable = task_group["taskSpec"]["runnables"][0]["container"]
    assert runnable["imageUri"] == "${KAIRO_IMAGE}"
    assert runnable["commands"][:2] == ["run", "--scenario"]
    assert "--output" in runnable["commands"]
    variables = task_group["taskSpec"]["environment"]["variables"]
    assert variables["KAIRO_OUTPUT_URI"] == "${KAIRO_OUTPUT_URI}"
    assert variables["KAIRO_CHECKPOINT_DIR"] == "/tmp/kairo/checkpoints"
    assert gcp_array["taskGroups"][0]["taskSpec"]["runnables"][0]["container"]["commands"][2].endswith("variant-${BATCH_TASK_INDEX}.yaml")
    assert gcp_array["taskGroups"][0]["taskCount"] == "${KAIRO_SWEEP_SIZE}"
    assert gcp_array["taskGroups"][0]["parallelism"] == "${KAIRO_PARALLELISM}"
    gcp_submitter = read("cloud/gcp/submit-experiment.sh")
    assert "KAIRO_SWEEP_SIZE required for GCP array jobs" in gcp_submitter
    assert 'task_group["taskCount"] = sweep_size' in gcp_submitter
    assert 'task_group["parallelism"] = min(parallelism, sweep_size)' in gcp_submitter

    azure_job = read_json("cloud/azure/batch-job.json")
    azure_array = read_json("cloud/azure/batch-array.json")
    assert azure_job["id"] == "kairo-ecs-job"
    assert azure_job["onAllTasksComplete"] == "terminateJob"
    assert azure_array["metadata"][0]["name"] == "sweepSize"
    submitter = read("cloud/azure/submit-experiment.ps1")
    assert "az batch job create" in submitter
    assert "az batch task create" in submitter
    assert "KAIRO_OUTPUT_URI=$OutputUri" in submitter
    assert "KAIRO_CHECKPOINT_DIR=/tmp/kairo/checkpoints" in submitter


def validate_k8s_manifests() -> None:
    crd = read("k8s/crd/kairoecs-experiment.yaml")
    assert "apiVersion: apiextensions.k8s.io/v1" in crd
    assert "kind: CustomResourceDefinition" in crd
    assert "KairoECSExperiment" in crd
    assert "required: [image, scenarioRef, storage]" in crd
    assert "enum: [s3, gcs, azure, filesystem]" in crd
    assert "subresources:" in crd and "status: {}" in crd

    sample = read_json("k8s/samples/experiment.json")
    assert sample["kind"] == "KairoECSExperiment"
    assert sample["spec"]["parallelism"] >= 1
    assert sample["spec"]["storage"]["backend"] in {"filesystem", "s3", "gcs", "azure"}
    validate_rendered_k8s_job()


def validate_rendered_k8s_job() -> None:
    job = run_json(
        [
            sys.executable,
            "k8s/operator/kairoecs_operator.py",
            "--experiment",
            "k8s/samples/experiment.json",
        ]
    )

    assert job["apiVersion"] == "batch/v1"
    assert job["kind"] == "Job"
    spec = job["spec"]
    assert spec["parallelism"] == 2
    assert spec["completionMode"] == "Indexed"
    container = spec["template"]["spec"]["containers"][0]
    env = {entry["name"]: entry["value"] for entry in container["env"]}
    assert env["KAIRO_STORAGE_BACKEND"] == "filesystem"
    assert env["KAIRO_OUTPUT_URI"] == "/var/lib/kairo/output"
    assert env["KAIRO_CHECKPOINT_ENABLED"] == "true"
    assert container["args"][:2] == ["run", "--scenario"]
    assert "--output" in container["args"]
    assert container["volumeMounts"][0]["mountPath"] == "/scenario"
    assert spec["template"]["spec"]["volumes"][0]["configMap"]["name"] == "factory-bottleneck-scenario"

    status = run_json(
        [
            sys.executable,
            "k8s/operator/kairoecs_operator.py",
            "--experiment",
            "k8s/samples/experiment.json",
            "--status",
        ]
    )
    assert status["status"]["phase"] == "Rendered"
    assert "observedGeneration" not in status["status"]
    assert status["status"]["completedRuns"] == 0
    assert status["status"]["failedRuns"] == 0

    inline_sample = json.loads(read("k8s/samples/experiment.json"))
    inline_sample["spec"]["scenarioRef"] = {"inline": "scenario_id: inline-smoke\n"}
    validation_root = ROOT / "cloud" / "validation-work"
    validation_root.mkdir(parents=True, exist_ok=True)
    inline_path = validation_root / "k8s-inline-experiment.json"
    try:
        inline_path.write_text(json.dumps(inline_sample), encoding="utf-8")
        inline_job = run_json(
            [
                sys.executable,
                "k8s/operator/kairoecs_operator.py",
                "--experiment",
                str(inline_path),
            ]
        )
    finally:
        inline_path.unlink(missing_ok=True)
    inline_spec = inline_job["spec"]["template"]["spec"]
    assert inline_spec["volumes"][0]["emptyDir"] == {}
    assert inline_spec["initContainers"][0]["name"] == "write-inline-scenario"
    assert inline_spec["initContainers"][0]["env"][0]["value"] == "scenario_id: inline-smoke\n"
    assert inline_spec["containers"][0]["args"][2] == "/scenario/scenario.yaml"


def validate_slurm_scripts() -> None:
    single = read("hpc/slurm/submit-experiment.sh")
    sweep = read("hpc/slurm/submit-sweep.sh")
    resume = read("hpc/slurm/resume.sh")
    for path, text in {
        "hpc/slurm/submit-experiment.sh": single,
        "hpc/slurm/submit-sweep.sh": sweep,
        "hpc/slurm/resume.sh": resume,
    }.items():
        assert "set -euo pipefail" in text, path

    assert "#SBATCH --signal=B:SIGTERM@120" in single
    assert 'KAIRO_CHECKPOINT_DIR="\\${KAIRO_CHECKPOINT_DIR:-${TMPDIR:-/tmp}/kairo/checkpoints}"' in single
    assert "kairo-ecs-cli checkpoint" in single
    assert "#SBATCH --array=0-${last_index}" in sweep
    assert "#SBATCH --signal=B:SIGTERM@120" in sweep
    assert "SLURM_ARRAY_TASK_ID" in sweep
    assert 'kairo-ecs-cli run --scenario "${scenario_prefix}/variant-\\${variant}.yaml" --output "\\$KAIRO_OUTPUT_URI"' in sweep
    assert "kairo-ecs-cli resume --checkpoint" in resume

    bash = shutil.which("bash")
    if bash:
        try:
            probe = subprocess.run([bash, "--version"], cwd=ROOT, text=True, capture_output=True)
        except OSError as exc:
            fallback_static_shell_validation(str(exc))
            return
        if probe.returncode != 0:
            stderr = probe.stderr.strip() or f"exit code {probe.returncode}"
            fallback_static_shell_validation(stderr)
            return
        for script in SHELL_SCRIPTS:
            subprocess.run([bash, "-n", script], cwd=ROOT, check=True)
    else:
        fallback_static_shell_validation("bash was not found on PATH")


def validate_policy_docs() -> None:
    policy = read("docs/cloud-hpc/checkpoint-spot-policy.md")
    assert "Offline validator scope" in policy
    assert "Live provider validation" in policy
    assert "local or shared POSIX filesystem path" in policy
    assert "does not prove" in policy
    for path in [
        "docs/cloud-hpc/aws-batch.md",
        "docs/cloud-hpc/gcp-batch.md",
        "docs/cloud-hpc/azure-batch.md",
        "docs/cloud-hpc/slurm.md",
    ]:
        text = read(path)
        assert "Offline validation" in text, path
        assert "Live validation" in text, path


def validate_telemetry_plugin() -> None:
    validation_root = ROOT / "cloud" / "validation-work"
    validation_root.mkdir(parents=True, exist_ok=True)
    source = validation_root / "events.arrow"
    target = validation_root / "out"
    source.write_bytes(b"kairo-ecs-arrow-smoke\n")

    copied = run_json(
        [
            sys.executable,
            "docker/telemetry-plugin/cloud-output.py",
            "--input",
            str(source),
            "--destination",
            validation_root_uri(target),
        ]
    )
    copied_path = Path(copied["copied"])
    checksum_path = Path(copied["checksum"])
    assert copied_path.is_file()
    assert checksum_path.is_file()
    checksum_text = checksum_path.read_text(encoding="utf-8")
    assert copied["sha256"] in checksum_text
    assert copied_path.name in checksum_text

    provider = run_json(
        [
            sys.executable,
            "docker/telemetry-plugin/cloud-output.py",
            "--input",
            str(source),
            "--destination",
            "s3://kairo-smoke/runs/events.arrow",
        ]
    )
    manifest = json.loads(Path(provider["manifest"]).read_text(encoding="utf-8"))
    assert manifest["schema"] == "kairo.ecs.telemetry-upload.v1"
    assert manifest["provider"] == "s3"
    assert manifest["destination"] == "s3://kairo-smoke/runs/events.arrow"
    assert manifest["checksum_sha256"] == provider["sha256"]


def validation_root_uri(path: Path) -> str:
    return path.resolve().as_uri()


def main() -> int:
    validation_root = ROOT / "cloud" / "validation-work"
    if validation_root.exists():
        shutil.rmtree(validation_root)
    for path in [
        "cloud/gcp/batch-job.json",
        "cloud/gcp/batch-array.json",
        "cloud/azure/batch-job.json",
        "cloud/azure/batch-array.json",
        "k8s/samples/experiment.json",
    ]:
        json.loads((ROOT / path).read_text(encoding="utf-8"))

    require("docker/Dockerfile", ["kairo-ecs-cli", "ENTRYPOINT"])
    require("docker/entrypoint.sh", ["trap on_term TERM INT", "checkpoint-manifest.json"])
    require("crates/kairo-ecs-cli/src/main.rs", ['"run" =>', '"checkpoint" =>', '"resume" =>'])
    require("k8s/crd/kairoecs-experiment.yaml", ["KairoECSExperiment", "checkpoint", "completedRuns"])
    require("k8s/operator/kairoecs_operator.py", ["render_job", "batch/v1"])
    require("cloud/aws/batch-job-definition.yaml", ["resourceRequirements", "KAIRO_OUTPUT_URI"])
    require("hpc/slurm/submit-experiment.sh", ["#SBATCH", "--signal=B:SIGTERM@120"])
    require("docs/cloud-hpc/slurm.md", ["checkpoint", "sbatch"])
    validate_docker_surface()
    validate_cloud_manifests()
    validate_k8s_manifests()
    validate_slurm_scripts()
    validate_policy_docs()
    validate_telemetry_plugin()
    if validation_root.exists():
        shutil.rmtree(validation_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
