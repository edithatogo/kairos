"""Optional Gymnasium and PettingZoo adapters for the Rust queue model."""

from dataclasses import dataclass
from operator import index
from typing import ClassVar

from ._native import (
    NATIVE_REQUIRED,
    NativeEnvironmentUnavailable,
    NativeQueue,
    executable_path,
)

try:
    import gymnasium
    import numpy as np
except ImportError:
    gymnasium = None

try:
    from pettingzoo import ParallelEnv
except ImportError:
    ParallelEnv = object

GymEnv = gymnasium.Env if gymnasium is not None else object


@dataclass
class KairoGymConfig:
    observation_size: int = 3
    action_size: int = 2
    max_steps: int = 128

    def __post_init__(self) -> None:
        for name in ("observation_size", "action_size", "max_steps"):
            value = getattr(self, name)
            if type(value) is not int:
                raise TypeError(f"{name} must be an integer")
            if value <= 0:
                raise ValueError(f"{name} must be greater than zero")


def _observation(values):
    return np.asarray(values, dtype=np.uint32)


def _observation_space(queue):
    return gymnasium.spaces.Box(
        low=_observation([0, 0, 0]),
        high=_observation(queue.observation_high),
        dtype=np.uint32,
    )


class KairoGymEnv(GymEnv):
    """Single-agent queue where the selected Rust executable owns the episode."""

    metadata: ClassVar[dict] = {"render_modes": []}
    render_mode = None

    def __init__(self, config: KairoGymConfig | None = None, *, executable=None):
        executable = executable_path(executable)
        if gymnasium is None:
            raise NativeEnvironmentUnavailable(
                "Install the optional Gymnasium dependency"
            )
        config = KairoGymConfig() if config is None else config
        if (config.observation_size, config.action_size) != (3, 2):
            raise ValueError(
                "The native queue requires three observation fields and two actions"
            )
        self._queue = NativeQueue(executable, 1, config.max_steps)
        self.observation_space = _observation_space(self._queue)
        self.action_space = gymnasium.spaces.Discrete(2)

    def reset(self, *, seed=None, options=None):
        seed = None if seed is None else index(seed)
        observation = self._queue.reset(seed)
        super().reset(seed=seed)
        return _observation(observation), {}

    def step(self, action):
        step = self._queue.step([action])
        return (
            _observation(step["observation"]),
            step["rewards"][0],
            step["terminated"],
            step["truncated"],
            {},
        )

    def close(self):
        self._queue.close()


class PettingZooQueueEnv(ParallelEnv):
    """Ordered agents sharing the same Rust queue inside one child process."""

    metadata: ClassVar[dict] = {
        "name": "kairo_queue_v1",
        "render_modes": [],
        "is_parallelizable": True,
    }
    render_mode = None

    def __init__(self, agents=2, max_steps=128, *, executable=None):
        executable = executable_path(executable)
        if gymnasium is None or ParallelEnv is object:
            raise NativeEnvironmentUnavailable(
                "Install the optional PettingZoo dependency"
            )
        self._queue = NativeQueue(executable, agents, max_steps)
        self.possible_agents = [f"worker_{i}" for i in range(agents)]
        self.agents = []
        self.observation_spaces = {
            agent: _observation_space(self._queue) for agent in self.possible_agents
        }
        self.action_spaces = {
            agent: gymnasium.spaces.Discrete(2) for agent in self.possible_agents
        }

    def observation_space(self, agent):
        return self.observation_spaces[agent]

    def action_space(self, agent):
        return self.action_spaces[agent]

    def reset(self, seed=None, options=None):
        observation = self._queue.reset(seed)
        self.agents = self.possible_agents.copy()
        return (
            {agent: _observation(observation) for agent in self.agents},
            {agent: {} for agent in self.agents},
        )

    def step(self, actions):
        if actions.keys() != set(self.agents):
            raise ValueError("Supply one action per active queue agent")
        if not self.agents:
            return {}, {}, {}, {}, {}

        agents = self.agents
        step = self._queue.step([actions[agent] for agent in agents])
        if step["terminated"] or step["truncated"]:
            self.agents = []
        return (
            {agent: _observation(step["observation"]) for agent in agents},
            dict(zip(agents, step["rewards"], strict=True)),
            dict.fromkeys(agents, step["terminated"]),
            dict.fromkeys(agents, step["truncated"]),
            {agent: {} for agent in agents},
        )

    def close(self):
        self._queue.close()
        self.agents = []


def register_kairo_env(env_id: str = "KairoECS-v0", *, executable=None) -> str:
    executable = executable_path(executable)
    if gymnasium is None:
        raise NativeEnvironmentUnavailable("Install the optional Gymnasium dependency")
    # Probe the versioned binary before exposing it through the host registry.
    with KairoGymEnv(executable=executable):
        pass
    gymnasium.register(
        id=env_id,
        entry_point="kairo_gym:KairoGymEnv",
        kwargs={"executable": str(executable)},
    )
    return env_id


def build_observation_space(size: int):
    raise NativeEnvironmentUnavailable(NATIVE_REQUIRED)


def build_action_space(size: int):
    raise NativeEnvironmentUnavailable(NATIVE_REQUIRED)
