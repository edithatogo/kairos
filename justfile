set shell := ["bash", "-cu"]

dev-setup:
    rustup component add clippy rustfmt
    cargo install cargo-nextest --locked || true
    cargo install cargo-vet --locked || true

dev-validate:
    rustc --version
    cargo --version
    cargo nextest --version
    cargo vet --version
    python --version
    node --version
    go version
    julia --version
    Rscript --version
    dotnet --version
    just --version

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo nextest run --workspace --all-features

docs-bootstrap:
    cd website && npm ci

docs-build:
    cd website && npm ci && npm run build

docs-dev:
    cd website && npm ci && npm start

check-docs:
    cd website && npm ci && npm run build && test -f build/index.html && rg -n "docs/adr/|docs/api/api-review-template.md|docs/benchmarks/benchmark-policy.md|docs/community/|docs/design/api-review.md|docs/interoperability/standards-review.md|docs/release/|docs/research/|docs/trustworthy-simulation/" src/index.md

security:
    cargo deny check
    cargo audit

bindings-smoke:
    echo "Run Python/R/Julia/TS/C#/Go smoke tests when bindings exist."

validate-conductor:
    pwsh -NoProfile -File scripts/validate_conductor_setup.ps1

validate-tracks:
    pwsh -NoProfile -File scripts/validate_track_coverage.ps1

validate-track-docs:
    pwsh -NoProfile -File scripts/validate_track_docs_clean.ps1

validate-conformance:
    pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1
