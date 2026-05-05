import unittest

from kairo_gym import KairoGymEnv, register_kairo_env


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


if __name__ == "__main__":
    unittest.main()
