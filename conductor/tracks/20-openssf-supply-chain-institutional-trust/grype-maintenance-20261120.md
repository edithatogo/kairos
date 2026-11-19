# Grype source dependency gate — 20 November 2026

Bounded Track 20 maintenance with Track 13 CI and Track 15/42 release callsites.
No runtime, ABI, package name, registry policy or readiness status changes.

The exact source Cargo.lock inventory now has a critical Grype check before both
release evidence profiles and registry preflight. The actual-archive acquisition,
producer/consumer source equality, byte verification, Syft qualification and
nonpublication controls stay intact. A source scan does not replace archive or
binding qualification. Advisory PR/weekly scans record their exact checkout.

The isolated writer used the repository path claim and a bounded hashed context.
Local source inventory and workflow validation are recorded in the PR; hosted
Grype and full native release gates are pending. No phase or maturity promotion
is claimed. See `docs/security/grype.md` for inventory boundaries.
