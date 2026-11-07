"""Gymnasium API reservation; execution requires a qualified Rust environment.

The former Python state-transition demonstration was retired on 2026-11-07.
Importing configuration types remains supported. No environment is registered or
constructed until the native observation/action/transition contract is available.
"""

from dataclasses import dataclass
from typing import Optional


class NativeEnvironmentUnavailable(RuntimeError):
    """The Rust simulation-backed Gymnasium environment is not implemented."""


_NATIVE_REQUIRED = (
    "KairoGymEnv requires a qualified Rust simulation backend. The Python "
    "simulation core was retired on 2026-11-07; no fallback is supported. "
    "See Conductor tracks 109 and 110 for native environment qualification."
)


@dataclass
class KairoGymConfig:
    observation_size: int = 4
    action_size: int = 2
    max_steps: int = 128

    def __post_init__(self) -> None:
        for name in ("observation_size", "action_size", "max_steps"):
            value = getattr(self, name)
            if type(value) is not int:
                raise TypeError(f"{name} must be an integer")
            if value <= 0:
                raise ValueError(f"{name} must be greater than zero")


class KairoGymEnv:
    """Retired host engine entry point; never executes a Python simulation."""

    def __init__(self, config: Optional[KairoGymConfig] = None):
        raise NativeEnvironmentUnavailable(_NATIVE_REQUIRED)


def register_kairo_env(env_id: str = "KairoECS-v0") -> str:
    """Reject registration instead of advertising an unavailable environment."""
    raise NativeEnvironmentUnavailable(_NATIVE_REQUIRED)


def build_observation_space(size: int):
    raise NativeEnvironmentUnavailable(_NATIVE_REQUIRED)


def build_action_space(size: int):
    raise NativeEnvironmentUnavailable(_NATIVE_REQUIRED)
