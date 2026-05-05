from __future__ import annotations

from dataclasses import dataclass
from typing import Any

try:
    import gymnasium as gym
    from gymnasium import spaces
except ImportError:  # pragma: no cover - exercised when optional dependency is absent
    gym = None

    class _Box:
        def __init__(self, low: float, high: float, shape: tuple[int, ...], dtype: type[float]):
            self.low = low
            self.high = high
            self.shape = shape
            self.dtype = dtype

        def sample(self) -> list[float]:
            return [0.0 for _ in range(self.shape[0])]

    class spaces:  # type: ignore[no-redef]
        Box = _Box


@dataclass
class KairoGymConfig:
    observation_size: int = 4
    action_size: int = 2
    max_steps: int = 128

    def __post_init__(self) -> None:
        for name in ("observation_size", "action_size", "max_steps"):
            value = getattr(self, name)
            if not isinstance(value, int):
                raise TypeError(f"{name} must be an integer")
            if value <= 0:
                raise ValueError(f"{name} must be greater than zero")


class _BaseEnv:
    pass


if gym is not None:
    _BaseEnv = gym.Env


class KairoGymEnv(_BaseEnv):
    metadata = {"render_modes": ["ansi"]}

    def __init__(self, config: KairoGymConfig | None = None):
        self.config = config or KairoGymConfig()
        self.observation_space = build_observation_space(self.config.observation_size)
        self.action_space = build_action_space(self.config.action_size)
        self._step_count = 0
        self._state = [0.0 for _ in range(self.config.observation_size)]

    def reset(self, *, seed: int | None = None, options: dict[str, Any] | None = None):
        if seed is not None and gym is not None:
            super().reset(seed=seed)
        self._step_count = 0
        self._state = [0.0 for _ in range(self.config.observation_size)]
        return list(self._state), {"seed": seed, "options": options or {}}

    def step(self, action: Any):
        values = _coerce_action(action, self.config.action_size)
        self._step_count += 1
        for index, value in enumerate(values):
            self._state[index % len(self._state)] += value

        reward = -sum(abs(value) for value in self._state)
        terminated = False
        truncated = self._step_count >= self.config.max_steps
        info = {"step": self._step_count}
        return list(self._state), reward, terminated, truncated, info

    def render(self):
        return f"KairoGymEnv(step={self._step_count}, state={self._state})"

    def close(self):
        return None


def build_observation_space(size: int):
    return spaces.Box(low=-1.0e9, high=1.0e9, shape=(size,), dtype=float)


def build_action_space(size: int):
    return spaces.Box(low=-1.0, high=1.0, shape=(size,), dtype=float)


def register_kairo_env(env_id: str = "KairoECS-v0") -> str:
    if gym is not None:
        gym.register(id=env_id, entry_point="kairo_gym:KairoGymEnv")
    return env_id


def _coerce_action(action: Any, size: int) -> list[float]:
    if hasattr(action, "tolist"):
        action = action.tolist()
    if isinstance(action, (int, float)):
        action = [float(action)]
    if isinstance(action, (str, bytes)):
        raise TypeError("action must be numeric or an iterable of numeric values")
    values = [float(value) for value in action]
    if len(values) < size:
        values.extend([0.0] * (size - len(values)))
    return values[:size]
