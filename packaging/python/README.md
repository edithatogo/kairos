# Python Packaging

Track 06 currently validates the pure Python binding with pytest and compile checks.
Wheel and sdist publication remain gated on local temp-directory write access for
`python -m build`; native runtime artifacts remain gated on Track 02/15 packaging.
The default package stays dependency-light, with real Arrow table roundtrips enabled
through the optional `kairo-ecs[arrow]` extra when `pyarrow` is installed.
