# Julia Packaging Notes

Track 08 owns this directory.

The Julia package is intentionally local-only for this slice. Registry
publication, package-server automation, and native library artifact packaging
remain deferred to Track 15 and the Track 02 FFI artifact handoff.

Track 08 now exposes a dependency-light event-log smoke payload in
`bindings/julia` so package tests can cover the Arrow schema boundary before
Julia Artifacts and native Arrow.jl IPC packaging are enabled.
