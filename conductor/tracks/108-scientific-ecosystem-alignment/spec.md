# kairos delivery contract

Approved by user on 29 October 2026. This bounded track implements the
[coordinated programme](https://github.com/edithatogo/repository-standards/tree/7874f05132447c77f681fbca7ad5394066564aaf/conductor/tracks/scientific_ecosystem_alignment_20261029); it does not replace existing native
acceptance or other active track owners.

- Reconcile Track 107 and accepted source baseline with integrator before pin changes.
- Qualify minimal core and optional Arrow/Polars features without changing simulation semantics.
- Verify RNG replay, schema and buffer ownership against CareOps and Innovate consumers.

Use frontier-first qualified candidates and explicit retained-version exceptions.
Separate core, binding, backend, build, test-oracle and tooling dependencies.
Acceptance requires exact-source numerical and consumer tests, measured builds,
agent correctness/security review, fixes and repository integration. Planning or
static source checks are not native acceptance. Existing promises are retained
until a versioned migration passes; no paid execution or public release implied.

## Release implications

Any versioned API change requires a migration note and provider and consumer
compatibility evidence. Track registration is not a release or native acceptance.

## Blocked paths

Track 107 and the accepted CareOps consumer baseline remain prerequisites. No
provider pin, RNG fixture, core API or runtime implementation is authorized by
this planning record alone.
