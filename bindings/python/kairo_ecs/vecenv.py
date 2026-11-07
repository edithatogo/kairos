"""Reserved RL adapters; no synthetic Python environment is supported."""

from ._errors import UnsupportedNativeOperation


class ParallelVecEnv:
    """Unavailable until a model-backed Rust vector environment is qualified."""

    def __init__(self, num_envs: int = 1) -> None:
        raise UnsupportedNativeOperation(
            "ParallelVecEnv requires a qualified Rust model adapter; "
            "the synthetic Python environment is retired."
        )


class PettingZooParallelAdapter:
    """Unavailable until the Rust multi-agent adapter is qualified."""

    def __init__(self, agents: list[str] | None = None) -> None:
        raise UnsupportedNativeOperation(
            "PettingZooParallelAdapter requires a qualified Rust model adapter; "
            "the synthetic Python environment is retired."
        )
