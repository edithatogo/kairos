set shell := ["bash", "-cu"]

dev-setup:
    pwsh -NoProfile -File scripts/bootstrap.ps1 -SkipPython -SkipNpm

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
    npm --prefix website ci

docs-build:
    npm --prefix website ci
    npm --prefix website run build

docs-dev:
    npm --prefix website ci
    npm --prefix website start

docs-smoke:
    node scripts/dx/validate-docs-workflow.mjs

check-docs:
    node scripts/dx/validate-docs-workflow.mjs

toolchain-docs:
    node scripts/dx/validate-toolchain-docs.mjs

security:
    cargo deny check
    cargo audit

bindings-smoke:
    echo "Run Python/R/Julia/TS/C#/Go smoke tests when bindings exist."

validate-conductor:
    pwsh -NoProfile -File scripts/validate_conductor_setup.ps1
    pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
    pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1

validate-tracks:
    pwsh -NoProfile -File scripts/validate_track_coverage.ps1

validate-track-docs:
    pwsh -NoProfile -File scripts/validate_track_docs_clean.ps1

validate-conformance:
    pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1

validate-tracks21-27:
    node scripts/validation/validate-tracks21-27.mjs
