# KairoECS Risk Acceptance Policy

## Purpose

KairoECS is ambitious enough that unmanaged risk will look like progress until release time. This policy defines which risks must be mitigated, accepted, deferred, or used to block release.

## Severity

| Severity | Meaning | Release behavior |
|---|---|---|
| Critical | Can invalidate safety, release trust, or core thesis | Blocks release until mitigated |
| High | Can harm users, adoption, or compatibility | Blocks stable release unless explicitly accepted |
| Medium | Meaningful but containable | Must have owner and follow-up |
| Low | Informational or local | Track in issue backlog |

## Required red-team checkpoints

```text
- before first public repo announcement
- before first package publication
- before every release candidate
- before any API/ABI/schema stability promise
- after any major dependency or toolchain change
```

## Acceptance record

Every accepted High/Critical risk must document:

```text
risk description
impact
likelihood
owner
mitigation attempted
reason for accepting
expiry/review date
release surface affected
```
