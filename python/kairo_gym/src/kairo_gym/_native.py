"""Local stdio client for the feature-enabled Rust queue executable."""

import json
import os
import secrets
import selectors
import subprocess
import time
import weakref
from operator import index
from pathlib import Path


class NativeEnvironmentUnavailable(RuntimeError):
    """A qualified Rust queue process is required for execution."""


NATIVE_REQUIRED = (
    "KairoGymEnv requires a qualified Rust simulation backend. Supply the "
    "CLI built with rl-queue as executable; no fallback is supported."
)
REPLY_TIMEOUT_SECONDS = 5
MAX_REPLY_BYTES = 1024


def executable_path(executable):
    if executable is None:
        raise NativeEnvironmentUnavailable(NATIVE_REQUIRED)
    if os.name != "posix":
        raise NativeEnvironmentUnavailable(
            "The experimental native queue needs POSIX process pipes"
        )
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


def _read_frame(pipe):
    deadline = time.monotonic() + REPLY_TIMEOUT_SECONDS
    frame = bytearray()
    with selectors.DefaultSelector() as selector:
        selector.register(pipe, selectors.EVENT_READ)
        while len(frame) < MAX_REPLY_BYTES:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(remaining):
                raise TimeoutError("Native queue reply timed out")
            chunk = os.read(pipe.fileno(), MAX_REPLY_BYTES - len(frame))
            if not chunk:
                break
            frame.extend(chunk)
            if b"\n" in chunk:
                if frame.endswith(b"\n") and frame.count(b"\n") == 1:
                    return frame
                break
    raise ValueError("Invalid or incomplete native queue frame")


class NativeQueue:
    """Owns one child process and makes one blocking request at a time."""

    def __init__(self, executable, agents, max_ticks):
        if type(agents) is not int or not 1 <= agents <= 32:
            raise ValueError("The native queue supports between 1 and 32 agents")
        if type(max_ticks) is not int or not 1 <= max_ticks < 2**32:
            raise ValueError(
                "The native queue needs a positive unsigned 32-bit tick limit"
            )
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
            )
        except OSError as error:
            raise NativeEnvironmentUnavailable(NATIVE_REQUIRED) from error

        self._close = weakref.finalize(self, _close_process, self._process)
        self._agents = agents
        self._initialized = False
        try:
            ready = self._receive()
            bounds = (ready.get("max_waiting"), ready.get("arrival_ticks"))
            if (
                ready.keys() != {"model", "max_waiting", "arrival_ticks"}
                or ready["model"] != "finite-shared-queue-v1"
                or any(
                    type(value) is not int or not 0 < value < 2**32 for value in bounds
                )
            ):
                raise self._unavailable("Invalid native queue handshake")
            self.observation_high = (max_ticks, *bounds)
        except BaseException:
            self.close()
            raise

    def _unavailable(self, message):
        if self._close.alive:
            try:
                self._process.kill()
            finally:
                self.close()
        return NativeEnvironmentUnavailable(message)

    def _receive(self):
        try:
            reply = json.loads(_read_frame(self._process.stdout))
        except (OSError, ValueError) as error:
            raise self._unavailable(
                "Native queue did not send a complete reply"
            ) from error
        if not isinstance(reply, dict):
            raise self._unavailable("Invalid reply from native queue")
        return reply

    def _request(self, message):
        if not self._close.alive:
            raise NativeEnvironmentUnavailable("Native queue is closed")
        frame = (json.dumps(message, separators=(",", ":")) + "\n").encode("ascii")
        if len(frame) > 512:
            raise ValueError("Native queue command exceeds the frame limit")
        try:
            self._process.stdin.write(frame)
            self._process.stdin.flush()
        except OSError as error:
            raise self._unavailable("Native queue is unavailable") from error
        reply = self._receive()
        if reply.keys() == {"error"} and isinstance(reply["error"], str):
            raise ValueError(reply["error"])
        return reply

    def _valid_observation(self, observation):
        return (
            isinstance(observation, list)
            and len(observation) == len(self.observation_high)
            and all(
                type(value) is int and 0 <= value <= high
                for value, high in zip(observation, self.observation_high, strict=True)
            )
        )

    def reset(self, seed):
        if seed is None:
            if not self._initialized:
                seed = secrets.randbits(64)
        else:
            seed = index(seed)
            if not 0 <= seed < 2**64:
                raise ValueError("Native queue seed must be an unsigned 64-bit integer")
        reply = self._request({"op": "reset", "seed": seed})
        if reply.keys() != {"observation"} or not self._valid_observation(
            reply["observation"]
        ):
            raise self._unavailable("Invalid reset reply from native queue")
        self._initialized = True
        return reply["observation"]

    def step(self, actions):
        reply = self._request(
            {"op": "step", "actions": [index(action) for action in actions]}
        )
        rewards = reply.get("rewards")
        if (
            reply.keys() != {"observation", "rewards", "terminated", "truncated"}
            or not self._valid_observation(reply["observation"])
            or not isinstance(rewards, list)
            or len(rewards) != self._agents
            or any(
                type(reward) not in (float, int)
                or not -self.observation_high[1] <= reward <= 5
                for reward in rewards
            )
            or type(reply["terminated"]) is not bool
            or type(reply["truncated"]) is not bool
        ):
            raise self._unavailable("Invalid step reply from native queue")
        return reply

    def close(self):
        self._close()
