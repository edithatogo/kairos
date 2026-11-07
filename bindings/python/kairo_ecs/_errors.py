"""Explicit errors at the Rust-only execution boundary."""


class NativeBackendUnavailable(RuntimeError):
    """The configured Rust artifact cannot supply the required native ABI."""


class UnsupportedNativeOperation(RuntimeError):
    """An operation has no supported Rust implementation in this binding."""


class NativeExecutionError(RuntimeError):
    """The Rust engine rejected an operation or has been closed."""
