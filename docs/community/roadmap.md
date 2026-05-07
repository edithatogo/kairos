# Public Roadmap

| Stage | Public label | Community promise | Gate |
|---|---|---|---|
| Stage 0 | foundation | Naming, governance, CI, and core contracts are visible. | Foundation track evidence exists. |
| Stage 1 | alpha | First useful engine slice can be inspected with deterministic scheduler and ECS state docs. | Core and conformance gates pass locally. |
| Stage 2 | preview | First user-facing package path, Arrow event logs, and examples are documented. | Package, docs, and example gates pass. |
| Stage 3 | preview | Multi-language binding previews are discoverable with clear native-FFI status. | Binding track handoffs name smoke checks. |
| Stage 4 | beta | OpenSSF trust surfaces, model zoo, compatibility table, and community onboarding are cross-linked. | `onboarding-docs`, release, security, conformance, and benchmark gates pass. |
| Stage 5 | stable | Selected APIs, C ABI, and telemetry schemas have compatibility commitments. | Compatibility and release-candidate gates pass. |

## Maturity labels

- `toy`: learning-only model or example.
- `reference`: intended as a readable example of a supported pattern.
- `validated`: has evidence against a conformance or domain fixture.
- `benchmark`: used for benchmark or regression comparison.
- `domain-preview`: realistic domain shape but not production guidance.

No community page should imply `stable` maturity unless the relevant release and compatibility gates have passed.

## Documentation system

The documentation system roadmap should use Astro Starlight as the docs
framework, with versioned docs handled by `starlight-versions`.

Recommended plugins:

- `starlight-links-validator` for docs link hygiene and broken-link checks.
- `starlight-versions` for versioned documentation releases and release-line navigation.
- `starlight-typedoc` if TypeScript API documentation is generated from source.
- `starlight-openapi` if API references are published from OpenAPI or Swagger definitions.
- `Algolia DocSearch` only if the built-in Pagefind search becomes insufficient for scale or discovery.

The site can keep using Starlight defaults for the rest of the baseline behavior:
navigation, code highlighting, dark mode, frontmatter validation, and markdown/MDX content.
