import unittest

from kairo_gym import KairoGymConfig, KairoGymEnv, register_kairo_env


class KairoGymContractTests(unittest.TestCase):
    def test_reset_step_close_contract(self):
        env = KairoGymEnv()

        observation, info = env.reset(seed=7)
        self.assertEqual(len(observation), 4)
        self.assertEqual(info["seed"], 7)

        observation, reward, terminated, truncated, info = env.step([0.5, -0.25])
        self.assertEqual(len(observation), 4)
        self.assertIsInstance(reward, float)
        self.assertFalse(terminated)
        self.assertFalse(truncated)
        self.assertEqual(info["step"], 1)
        self.assertTrue(env.render().startswith("KairoGymEnv"))
        self.assertIsNone(env.close())

    def test_register_returns_env_id_without_optional_dependency(self):
        self.assertEqual(register_kairo_env("KairoECS-Test-v0"), "KairoECS-Test-v0")

    def test_config_rejects_invalid_space_sizes(self):
        with self.assertRaisesRegex(ValueError, "observation_size must be greater than zero"):
            KairoGymConfig(observation_size=0)

        with self.assertRaisesRegex(TypeError, "action_size must be an integer"):
            KairoGymConfig(action_size=1.5)  # type: ignore[arg-type]

    def test_step_rejects_string_actions(self):
        env = KairoGymEnv()
        env.reset()

        with self.assertRaisesRegex(TypeError, "action must be numeric"):
            env.step("bad-action")


if __name__ == "__main__":
    unittest.main()
