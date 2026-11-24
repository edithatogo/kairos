"""Run the native Track 55 ring on a Mac and a loopback-forwarded NAS worker."""
from contextlib import ExitStack
import json
from pathlib import Path
import secrets
import select
import shlex
import socket
import subprocess

ROOT = Path(__file__).resolve().parent
OUTPUT = ROOT / "evidence"
MAC = ROOT / "mac-target/release/examples/track55_physical"
NAS_BINARY = (ROOT / "remote-stage.private").read_text().strip() + "/track55_physical"
SSH = [
    "ssh", "-T",
    "-o", "BatchMode=yes",
    "-o", "StrictHostKeyChecking=yes",
    "-o", "ConnectTimeout=5",
    "-o", "ControlMaster=no",
    "-o", "ControlPath=none",
    "-o", "ServerAliveInterval=5",
    "-o", "ServerAliveCountMax=2",
    "-o", "ExitOnForwardFailure=yes",
]
ALIAS = "pleiadesnas"


def local_ports(count):
    with ExitStack() as stack:
        listeners = [stack.enter_context(socket.socket()) for _ in range(count)]
        for listener in listeners:
            listener.bind(("127.0.0.1", 0))
        return [listener.getsockname()[1] for listener in listeners]


def wait_ready(process, expected):
    ready, _, _ = select.select([process.stdout], [], [], 10)
    value = process.stdout.readline().strip() if ready else ""
    if value != expected:
        raise RuntimeError(f"Peer startup {expected!r} observed {value!r}")
    return value


def terminate(process):
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def run():
    controller, local, forward = local_ports(3)
    remote_worker, reverse = (secrets.randbelow(10_000) + offset for offset in (20_000, 40_000))
    remote_check = (
        f"ss -H -ltn 'sport = :{reverse}' | "
        f"""awk '$4 == "127.0.0.1:{reverse}" {{good++}} END {{exit !(NR == 1 && good == 1)}}'"""
    )
    remote_job = shlex.join([
        "/usr/bin/timeout", "90", "/usr/bin/time", "-v", NAS_BINARY,
        "worker", "1", f"127.0.0.1:{remote_worker}", f"2=127.0.0.1:{reverse}",
    ])
    remote_command = f"{remote_check} && exec {remote_job}"
    argv = {
        "mac_worker": ["/usr/bin/time", "-l", str(MAC), "worker", "0",
                       f"127.0.0.1:{local}", f"2=127.0.0.1:{controller}"],
        "nas_worker": [*SSH, "-L", f"127.0.0.1:{forward}:127.0.0.1:{remote_worker}",
                       "-R", f"127.0.0.1:{reverse}:127.0.0.1:{controller}", ALIAS, remote_command],
        "controller": ["/usr/bin/time", "-l", str(MAC), "controller",
                       f"127.0.0.1:{controller}", f"0=127.0.0.1:{local}", f"1=127.0.0.1:{forward}"],
    }
    commands = OUTPUT / "commands.private.json"
    commands.write_text(json.dumps(argv, indent=2) + "\n")
    ready = []
    statuses = {}
    with ExitStack() as stack:
        workers = {}
        for index, name in enumerate(("mac_worker", "nas_worker")):
            stderr = stack.enter_context((OUTPUT / f"{name}.stderr").open("w"))
            proc = subprocess.Popen(argv[name], text=True, stdout=subprocess.PIPE, stderr=stderr)
            stack.callback(proc.stdout.close)
            stack.callback(terminate, proc)
            workers[name] = proc
            ready.append(wait_ready(proc, f"ready {index}"))
        (OUTPUT / "workers-ready.txt").write_text("\n".join(ready) + "\n")
        with (OUTPUT / "measurements.jsonl").open("w") as data, (OUTPUT / "controller.stderr").open("w") as stderr:
            statuses["controller"] = subprocess.run(
                argv["controller"], stdout=data, stderr=stderr, check=False, timeout=64,
            ).returncode
        for name, proc in workers.items():
            statuses[name] = proc.wait(timeout=8)
    (OUTPUT / "exit.json").write_text(json.dumps(statuses, indent=2) + "\n")
    if statuses != {"controller": 0, "mac_worker": 0, "nas_worker": 0}:
        raise RuntimeError(statuses)


if __name__ == "__main__":
    run()
