"""Check the adapters against the feature-enabled native executable."""

import gc
import json
import os
import subprocess
import unittest
from pathlib import Path

from kairo_gym import KairoGymConfig, KairoGymEnv, NativeEnvironmentUnavailable, PettingZooQueueEnv


class NativeQueueTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        executable = os.environ.get("KAIRO_NATIVE_QUEUE_BIN")
        if executable is None:
            raise unittest.SkipTest("KAIRO_NATIVE_QUEUE_BIN must name a built rl-queue executable")
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

        with KairoGymEnv(KairoGymConfig(max_steps=1), executable=self.executable) as env:
            child = env._queue._process
            observed, _ = env.reset(seed=42)
            self.assertEqual(observed.tolist(), initial["observation"])
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
