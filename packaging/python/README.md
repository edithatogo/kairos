# Python packaging

The package provides a Rust C ABI adapter plus host DTO/data conversion helpers.
It no longer contains an executable Python scheduler or synthetic environment.
A wheel/sdist build does not establish native execution capability.

Publication must pair the Python package with an identified Rust artifact,
verified ABI/platform metadata and the required NativeEngine suite described
in [the binding guide](../../bindings/python/README.md). Until native runtime
assets are packaged and qualified, installation requires an explicit absolute
shared-library path. Arrow conversion tests remain a separate data gate.
