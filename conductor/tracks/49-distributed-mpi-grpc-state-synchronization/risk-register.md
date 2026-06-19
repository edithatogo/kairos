# Track 49 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Local emulator remains in production path | False distributed claim | Gate `Done` on real MPI and gRPC runtime tests |
| MPI implementation differences | Non-portable runtime | Record MPI vendor/version in evidence manifests |
| gRPC schema evolves without compatibility | Wire breakage | Protobuf compatibility tests and versioned messages |
| Migration loses component generations | State corruption | Generation-aware migration tests |
| Network failure handling hides data loss | Incorrect results | Failure classification and telemetry evidence |
