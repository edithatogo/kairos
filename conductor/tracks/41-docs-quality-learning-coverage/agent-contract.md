# 41 Documentation Platform, Quality Gates & Learning Coverage -- agent-contract.md

## Scope

This track coordinates the remaining strictness and learning-coverage work
across the repo. It is a cross-cutting docs/CI/community track, not a core
engine or binding implementation track.

## Primary responsibilities

- Tighten CI policy where the repo already has concrete tooling and a
  meaningful failure mode.
- Move the docs stack toward the Astro/Starlight roadmap without losing the
  current docs tree entry points.
- Produce a coverage matrix for tutorials, examples, and notebooks across the
  supported languages and example families.
- Keep notebook/tutorial inventories aligned with the site navigation and
  community onboarding pages.

## Working rules

- Do not modify `crates/` or `bindings/` implementation code without a handoff
  from the owning track.
- Prefer reusable policy or inventory files over one-off notes.
- Record every deliberate exclusion in the handoff, not in chat only.
- Keep strictness changes explicit: if a lane stays permissive, say why.

## Parallel lanes

- CI strictness lane: workflows and policy docs.
- Docs platform lane: website and docs navigation.
- Learning coverage lane: tutorials, examples, notebooks, and inventory.

## Deliverables

- A repo-level strictness policy statement for concrete CI surfaces.
- A docs migration or parity plan tied to the current site.
- A language/example coverage matrix with notebook exclusions justified.
- A completed handoff with validation notes and known gaps.
