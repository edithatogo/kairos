# Exact Rust setup selection: local static evidence

Source de1bc5aded2506b568e55426427f2dd30a882bea pins mise to 1.99.0 and enforces installed exact Rust cargo/rustc/rustdoc selection before workspace tests. Independent review verified version guards, locked argv, cleared wrappers and environment restoration.

All three recorded static gates passed and their raw log/source hashes were rechecked by the coordinator. PowerShell regression mocks extract the actual production functions. These gates execute no Rust tooling; Windows native behaviour, workspace runtime and hosted acceptance remain unverified. Conditional Rust CI routing is unchanged. Full C2 remains open.
