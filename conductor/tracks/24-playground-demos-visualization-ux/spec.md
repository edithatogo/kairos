# Track 24: Playground, Demos & Visualization UX

## Purpose

Build web demos and interactive visualization stories that make KairoECS understandable in minutes — live event timelines, simulation viewers, and browser-accessible playgrounds.

## Why this track exists

KairoECS is a multi-language research and engineering ecosystem. Without a playground and interactive demos, users evaluating the project must clone, build, and run examples before understanding what the kernel can do. This track lowers the evaluation barrier.

## Primary subagent

`playground-agent`

## Dependencies

- Track 05: Visualization — needed for rendering event timelines and simulation state viewers.
- Track 09: TypeScript/Wasm binding — needed for browser-side execution.
- Track 14: Documentation site — needed for hosting playground and demo pages.

## Owned paths

```text
website/, examples/viz/
```

## Blocked paths

```text
crates/kairo-ecs-viz/ — owned by Track 05 (visualization)
bindings/typescript/ — owned by Track 09 (Wasm binding)
```

## Inputs

- Visualization snapshot contract from Track 05.
- Wasm build output from Track 09.
- Docs site layout and deployment from Track 14.

## Outputs

- Playground page on the docs site with embedded Wasm demo.
- Visualization story pages showing event timelines and simulation viewers.
- Screenshot assets for docs site demo pages.
- Release gate row in `conductor/delivery-readiness-checklist.md` for demo availability.

## Acceptance criteria

- Playground page loads under 5s on simulated 4G connection.
- No console errors on playground page load.
- Screenshot PNGs exist at expected paths before beta.
- Demo pages are linked from the docs home page.
- Layout accessibility sanity check passes (keyboard navigation, contrast ratio).

## Non-goals

- Replacing the core scheduler or ECS design.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Shipping production visualization dashboards before kernel trust is established.

## Release implications

- Playground and demo presence is a docs-readiness gate.
- Misleading demo visuals (claiming features not yet implemented) block release.
- Screenshot-driven docs must be regenerated when visualization changes.
