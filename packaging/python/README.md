# Python Packaging

Track 06 currently validates the pure Python binding with pytest and compile checks.
Wheel and sdist publication remain gated on installing the local `build` module and
adding native runtime artifacts once the FFI library is packaged.
