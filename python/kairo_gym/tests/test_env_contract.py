"""Negative support tests; these do not qualify a native Gymnasium runtime."""

import unittest
from unittest.mock import patch

from kairo_gym import (
    KairoGymConfig,
    KairoGymEnv,
    NativeEnvironmentUnavailable,
    register_kairo_env,
)
from kairo_gym.env import build_action_space, build_observation_space


class KairoGymContractTests(unittest.TestCase):
    def test_constructor_cannot_run_retired_python_core(self):
        with self.assertRaisesRegex(NativeEnvironmentUnavailable, "Rust simulation backend"):
            KairoGymEnv()

    def test_registration_cannot_advertise_unavailable_environment(self):
        with self.assertRaisesRegex(NativeEnvironmentUnavailable, "no fallback"):
            register_kairo_env("KairoECS-Test-v0")

    def test_environment_variables_do_not_enable_a_host_fallback(self):
        with patch.dict("os.environ", {"KAIRO_ECS_ALLOW_FALLBACK": "1"}):
            with self.assertRaises(NativeEnvironmentUnavailable):
                KairoGymEnv(KairoGymConfig())

    def test_spaces_do_not_silently_fake_optional_gymnasium(self):
        for factory in (build_action_space, build_observation_space):
            with self.subTest(factory=factory.__name__):
                with self.assertRaises(NativeEnvironmentUnavailable):
                    factory(4)

    def test_config_remains_a_data_contract(self):
        config = KairoGymConfig(observation_size=3, action_size=1, max_steps=20)
        self.assertEqual((config.observation_size, config.action_size, config.max_steps), (3, 1, 20))

    def test_config_rejects_invalid_sizes(self):
        for invalid in (False, 1.5):
            with self.assertRaises(TypeError):
                KairoGymConfig(action_size=invalid)
        with self.assertRaises(ValueError):
            KairoGymConfig(observation_size=0)


if __name__ == "__main__":
    unittest.main()
