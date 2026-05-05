# 14 Documentation Site & Education — spec.md

## Mission

Create the public docs site in `website/`, wire it to the repository docs tree, and keep tutorials, API references, and examples discoverable from one entry point.

## Primary subagent

```text
docs-agent
```

## Dependencies

```text
Track 00. Starts immediately from specs.
```

## Owned paths

```text
docs, website, examples/docs, templates/website
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Website scaffold from justfile, docs tree layout from repo, API surface inventory from Tracks 01-05, binding documentation from Tracks 06-11.

## Outputs

- Built GitHub Pages site, install page, quickstarts for Rust/Python/Wasm, factory bottleneck tutorial, flocking behavior tutorial, API docs, release compatibility docs.

## Blocked paths

- `crates/` — owned by Tracks 01–05.
- `bindings/` — owned by Tracks 06–11.

## Docs scope

Docs must include:

```text
conceptual intro to KairoECS
DES tutorial: factory bottleneck
ABM tutorial: flocking behavior
hybrid tutorial: agents entering queues/resources
API quickstarts for every language
FFI and Arrow schema reference
performance guide
release and compatibility guide
governance/contribution guide
```

The site home should point at the current repository layout, not a stale external docs framework.



## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.



