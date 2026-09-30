# 42 Package Registry Publication & Provenance - agent-contract.md

## publication-agent

Owns `packaging/publication-registry-manifest.json`, `.github/workflows/registry-publish.yml`, and publication validation.

Must not perform a public registry write unless:

- the workflow is manually dispatched with `publish=true`
- the protected `release-publication` environment approves the run
- Track 44 reports code/repo health `>= 9.5`
- registry owner/trusted-publisher setup exists
- release-manager approval is recorded

## Binding agents

Each binding agent owns package-specific metadata, tests, docs, and conformance proof for its ecosystem.

## release-agent

Owns final publication approval, version selection, rollback/yank plan, and release notes.
