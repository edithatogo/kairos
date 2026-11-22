"""Check the adapters against the feature-enabled native executable."""

import gc
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from kairo_gym import (
    KairoGymConfig,
    KairoGymEnv,
    NativeEnvironmentUnavailable,
    PettingZooQueueEnv,
    _native,
    register_kairo_env,
)


class NativeQueueTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        executable = os.environ.get("KAIRO_NATIVE_QUEUE_BIN")
        if executable is None:
            raise unittest.SkipTest(
                "KAIRO_NATIVE_QUEUE_BIN must name a built rl-queue executable"
            )
        cls.executable = Path(executable).resolve(strict=True)

    def test_gymnasium_checker_uses_the_native_queue(self):
        from gymnasium.utils.env_checker import check_env

        with KairoGymEnv(executable=self.executable) as env:
            check_env(env, skip_render_check=True)

    def test_pettingzoo_parallel_checker_uses_the_native_queue(self):
        from pettingzoo.test import parallel_api_test

        env = PettingZooQueueEnv(agents=2, max_steps=10, executable=self.executable)
        self.addCleanup(env.close)
        parallel_api_test(env, num_cycles=40)

    def test_registered_gymnasium_environment_runs_the_selected_native_binary(self):
        import gymnasium

        env_id = register_kairo_env("KairoQueueTest-v1", executable=self.executable)
        with gymnasium.make(env_id) as env:
            initial, _ = env.reset(seed=42)
            self.assertEqual(initial.tolist(), [0, 2, 8])
            self.assertEqual(env.step(1)[0][0], 1)

    def test_gym_values_match_direct_native_wire_and_survive_reset_and_close(self):
        commands = [{"op": "reset", "seed": 42}, {"op": "step", "actions": [1]}]
        frames = "".join(json.dumps(command) + "\n" for command in commands)
        checked = subprocess.run(
            [str(self.executable), "queue-env-v1", "--agents", "1", "--max-ticks", "1"],
            input=frames,
            text=True,
            capture_output=True,
            check=True,
            timeout=10,
        )
        model, initial, transition = map(json.loads, checked.stdout.splitlines())
        self.assertEqual(model["model"], "finite-shared-queue-v1")

        with KairoGymEnv(
            KairoGymConfig(max_steps=1), executable=self.executable
        ) as env:
            child = env._queue._process
            observed, _ = env.reset(seed=42)
            self.assertEqual(observed.tolist(), initial["observation"])
            with self.assertRaisesRegex(ValueError, "unsigned 64-bit integer"):
                env.reset(seed=2**64)
            self.assertEqual(env.np_random_seed, 42)
            with self.assertRaisesRegex(ValueError, "one action"):
                env.step(2)
            step, reward, terminated, truncated, _ = env.step(1)
            self.assertEqual(step.tolist(), transition["observation"])
            self.assertEqual(reward, transition["rewards"][0])
            self.assertFalse(terminated)
            self.assertTrue(truncated)
            with self.assertRaisesRegex(ValueError, "episode is finished"):
                env.step(1)
            env.reset(seed=42)
            self.assertEqual(env.step(1)[0].tolist(), step.tolist())

        self.assertEqual(child.returncode, 0)
        self.assertEqual(observed.tolist(), [0, 2, 8])
        self.assertEqual(step.tolist(), transition["observation"])
        with self.assertRaisesRegex(NativeEnvironmentUnavailable, "closed"):
            env.reset(seed=42)
        env.close()

    def test_multi_agent_observations_are_owned_and_child_is_reaped_on_drop(self):
        env = PettingZooQueueEnv(agents=2, executable=self.executable)
        process = env._queue._process
        start, _ = env.reset(seed=42)
        a, b = env.agents
        start[a][1] = 123
        self.assertEqual(start[b].tolist(), [0, 2, 8])
        with self.assertRaisesRegex(ValueError, "one action per"):
            env.step({a: 1})
        obs, rewards, _, _, _ = env.step({a: 1, b: 1})
        self.assertEqual(obs[a][0], 1)
        waiting = int(obs[a][1])
        self.assertLessEqual(waiting, 1)
        self.assertEqual(rewards, {a: 5.0 - waiting, b: 5.0 - waiting})
        del env
        gc.collect()
        self.assertEqual(process.returncode, 0)
        self.assertEqual(obs[b][0], 1)


@unittest.skipUnless(os.name == "posix", "native queue uses POSIX process pipes")
class InvalidNativeQueueTests(unittest.TestCase):
    def test_bad_handshakes_time_out_or_fail_and_reap_the_child(self):
        ready = '{"model":"finite-shared-queue-v1","max_waiting":10,"arrival_ticks":8}'
        cases = [
            ("", "complete reply"),
            (f"printf '%s' '{ready}'\n", "complete reply"),
            (
                "printf '%s\\n' '{\"model\":\"finite-shared-queue-v1\"}'\n",
                "invalid native queue handshake",
            ),
            (
                f"printf '%s\\n' '{ready.replace('10', '-1')}'\n",
                "invalid native queue handshake",
            ),
        ]
        real_popen = subprocess.Popen
        launched = []

        def record_child(*args, **kwargs):
            process = real_popen(*args, **kwargs)
            launched.append(process)
            return process

        with (
            tempfile.TemporaryDirectory() as folder,
            patch.object(_native, "REPLY_TIMEOUT_SECONDS", 0.25),
            patch.object(_native.subprocess, "Popen", side_effect=record_child),
        ):
            executable = Path(folder) / "invalid-transport"
            for command, failure in cases:
                with self.subTest(command=command):
                    executable.write_text(f"#!/bin/sh\n{command}exec /bin/sleep 30\n")
                    executable.chmod(0o700)
                    with self.assertRaisesRegex(
                        NativeEnvironmentUnavailable, f"(?i){failure}"
                    ):
                        _native.NativeQueue(executable, 1, 10)
                    process = launched[-1]
                    self.assertIsNotNone(process.returncode)
                    self.assertTrue(process.stdin.closed and process.stdout.closed)

    def test_invalid_native_transition_is_rejected_before_host_values_escape(self):
        ready = '{"model":"finite-shared-queue-v1","max_waiting":10,"arrival_ticks":8}'
        with tempfile.TemporaryDirectory() as folder:
            executable = Path(folder) / "invalid-transition"
            executable.write_text(
                f"#!/bin/sh\nprintf '%s\\n' '{ready}'\n"
                "IFS= read -r line\nprintf '%s\\n' '{\"observation\":[1,2,7]}'\n"
                "exec /bin/sleep 30\n"
            )
            executable.chmod(0o700)
            queue = _native.NativeQueue(executable, 1, 10)
            self.addCleanup(queue.close)
            with self.assertRaisesRegex(
                NativeEnvironmentUnavailable, "Invalid step reply"
            ):
                queue.step([1])
            self.assertIsNotNone(queue._process.returncode)
