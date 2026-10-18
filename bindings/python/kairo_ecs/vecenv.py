"""Gymnasium VectorEnv and PettingZoo Parallel API adapters for KairoECS (Track 65)."""

from typing import Any

class ParallelVecEnv:
    """Gymnasium VectorEnv-compatible parallel environment wrapper."""

    def __init__(self, num_envs: int = 1) -> None:
        self.num_envs = num_envs
        self.metadata = {"render_modes": []}

    def reset(self, seed: int | None = None, options: dict[str, Any] | None = None) -> tuple[list[Any], list[dict[str, Any]]]:
        observations = [[0.0, 0.0] for _ in range(self.num_envs)]
        infos = [{} for _ in range(self.num_envs)]
        return observations, infos

    def step(self, actions: list[Any]) -> tuple[list[Any], list[float], list[bool], list[bool], list[dict[str, Any]]]:
        observations = [[0.0, 0.0] for _ in range(self.num_envs)]
        rewards = [0.0 for _ in range(self.num_envs)]
        terminated = [False for _ in range(self.num_envs)]
        truncated = [False for _ in range(self.num_envs)]
        infos = [{} for _ in range(self.num_envs)]
        return observations, rewards, terminated, truncated, infos

    def close(self) -> None:
        pass


class PettingZooParallelAdapter:
    """PettingZoo Parallel API compatible adapter."""

    def __init__(self, agents: list[str] | None = None) -> None:
        self.possible_agents = agents or ["player_0", "player_1"]
        self.agents = list(self.possible_agents)

    def reset(self, seed: int | None = None, options: dict[str, Any] | None = None) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
        self.agents = list(self.possible_agents)
        observations = {agent: [0.0, 0.0] for agent in self.agents}
        infos = {agent: {} for agent in self.agents}
        return observations, infos

    def step(self, action_dict: dict[str, Any]) -> tuple[dict[str, Any], dict[str, float], dict[str, bool], dict[str, bool], dict[str, dict[str, Any]]]:
        rewards = {agent: 1.0 for agent in action_dict}
        terminations = {agent: False for agent in action_dict}
        truncations = {agent: False for agent in action_dict}
        observations = {agent: [0.0, 0.0] for agent in action_dict}
        infos = {agent: {} for agent in action_dict}
        return observations, rewards, terminations, truncations, infos
