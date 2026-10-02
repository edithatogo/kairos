# CareOps D2 native owner CI handoff

Track 27 workflow integration only; Tracks 01–04 retain engine source ownership.
No API, scheduling, RNG, schema or release promise changes. The new workflow
checks actual host and commit, then tests locked core/types/state/rng/DES/ABM
packages on native Linux x86_64 and macOS ARM64. No secrets, cache or publication.
This is a bounded native consumer owner lane; full `just ci`, backend, binding,
coverage and global release checks remain independent and were not run here.
The parent must check its exact gitlink and accepted compatibility metadata;
a green owner workflow alone must never accept an incompatible parent pin.
