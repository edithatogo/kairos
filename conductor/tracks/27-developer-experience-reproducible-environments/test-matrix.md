# Test Matrix: Track 27 Developer Experience & Reproducible Environments

## Required tests

- `just docs-bootstrap`
- `just docs-build`
- `test -f website/build/index.html`
- `just validate-conductor`
- `npm start` in `website` starts a local preview on `http://localhost:3000`

## Required by release stage

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Docs bootstrap command | yes | yes | yes |
| Docs build command | yes | yes | yes |
| Docs preview command | no | yes | yes |
| Artifact existence check | yes | yes | yes |
| Contract compatibility check | no | yes | yes |
| Release gate integration | no | yes | yes |
| Cross-language conformance relevance reviewed | no | yes | yes |
| Red-team objections answered | yes | yes | yes |
