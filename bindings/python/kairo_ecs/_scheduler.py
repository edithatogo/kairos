"""Retired host-language scheduler; execution is exclusively Rust-native."""

from ._errors import UnsupportedNativeOperation


class Scheduler:
    """Retired Python scheduler; use :class:`kairo_ecs.NativeEngine`."""

    def __init__(self, *args: object, **kwargs: object) -> None:
        raise UnsupportedNativeOperation(
            "The Python scheduler is retired: only the Rust core is supported. "
            "Use NativeEngine with KAIRO_ECS_FFI_LIBRARY. Its ABI v1 API exposes "
            "opaque handles and native statistics, not the retired Python trace API."
        )
