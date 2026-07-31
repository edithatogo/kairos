import kairo_gym
from kairo_gym import KairoGymEnv

env = KairoGymEnv()
obs, info = env.reset()
for _ in range(10):
    action = env.action_space.sample()
    obs, reward, terminated, truncated, info = env.step(action)
    print(f"Step: {info['step']}, Reward: {reward}")
    if terminated or truncated:
        break
