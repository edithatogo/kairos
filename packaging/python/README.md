# Python Packaging

Track 06 currently validates the pure Python binding with pytest, compile checks,
and local wheel/sdist builds. The default package stays dependency-light, with
Arrow table roundtrips enabled through the optional `kairo-ecs[arrow]` extra when
`pyarrow` imports successfully. Native runtime artifacts remain gated on Track
02/15 packaging.

Local note from the 2026-05-08 closeout: `python -m build --sdist --wheel`
passes outside the sandbox with `TEMP`/`TMP` pointed at a package-local `.tmp`
directory. A workspace-local `pyarrow-24.0.0` install succeeds, but importing
`pyarrow.lib` fails with a Windows DLL-load error on this host, so the real Arrow
table roundtrip remains environment-blocked until the missing DLL/runtime
dependency is resolved.
