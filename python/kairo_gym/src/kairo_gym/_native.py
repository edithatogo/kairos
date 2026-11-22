"""Local stdio client for the feature-enabled Rust queue executable."""

import json
import secrets
import subprocess
import weakref
from operator import index
from pathlib import Path


class NativeEnvironmentUnavailable(RuntimeError):
    """A qualified Rust queue process is required for execution."""


NATIVE_REQUIRED = (
    "KairoGymEnv requires a qualified Rust simulation backend. Supply the "
    "cli built with rl-queue as executable; no fallback is supported."
)


def executable_path(executable):
    if executable is None:
        raise NativeEnvironmentUnavailable(NATIVE_REQUIRED)
    try:
        return Path(executable).resolve(strict=True)
    except OSError as error:
        raise NativeEnvironmentUnavailable(NATIVE_REQUIRED) from error


def _close_process(process):
    try:
        process.stdin.close()
    except BrokenPipeError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
    finally:
        process.stdout.close()


class NativeQueue:
    """Owns one child process and makes one blocking request at a time."""

    def __init__(self, executable, agents, max_ticks):
        if type(agents) is not int or not 1 <= agents <= 32:
            raise ValueError("The native queue supports between 1 and 32 agents")
        if type(max_ticks) is not int or not 1 <= max_ticks < 2**32:
            raise ValueError("The native queue needs a positive unsigned 32-bit tick limit")
        try:
            self._process = subprocess.Popen(
                [
                    str(executable_path(executable)),
                    "queue-env-v1",
                    "--agents",
                    str(agents),
                    "--max-ticks",
                    str(max_ticks),
                ],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                text=True,
                encoding="utf-8",
            )
        except OSError as error:
            raise NativeEnvironmentUnavailable(NATIVE_REQUIRED) from error

        self._close = weakref.finalize(self, _close_process, self._process)
        try:
            ready = self._receive()
            if ready.get("model") != "finite-shared-queue-v1":
                raise NativeEnvironmentUnavailable("Unexpected native queue model version")
            self.observation_high = (max_ticks, ready["max_waiting"], ready["arrival_ticks"])
        except BaseException:
            self.close()
            raise

    def _receive(self):
        try:
            reply = json.loads(self._process.stdout.readline(1024))
        except (OSError, ValueError) as error:
            raise NativeEnvironmentUnavailable("Native queue did not respond") from error
        if not isinstance(reply, dict):
            raise NativeEnvironmentUnavailable("Invalid reply from native queue")
        return reply

    def _request(self, message):
        if not self._close.alive:
            raise NativeEnvironmentUnavailable("Native queue is closed")
        frame = json.dumps(message, separators=(",", ":")) + "\n"
        if len(frame) > 512:
            raise ValueError("Native queue command exceeds the frame limit")
        try:
            self._process.stdin.write(frame)
            self._process.stdin.flush()
        except OSError as error:
            raise NativeEnvironmentUnavailable("Native queue is unavailable") from error
        reply = self._receive()
        if "error" in reply:
            raise ValueError(reply["error"])
        return reply

    def reset(self, seed):
        seed = secrets.randbits(64) if seed is None else index(seed)
        if not 0 <= seed < 2**64:
            raise ValueError("Native queue seed must be an unsigned 64-bit integer")
        return self._request({"op": "reset", "seed": seed})["observation"]

    def step(self, actions):
        return self._request({"op": "step", "actions": [index(action) for action in actions]})

    def close(self):
        self._close()
