# C1 exact transitive licence disposition

This isolated development slice adopts the reviewed candidate policy bytes for
`tiny-keccak =2.0.2` only, permitting `CC0-1.0`. The global allowlist, advisory,
ban and source policies are unchanged. Track 13 owns `deny.toml`; this records
the supply-chain handoff to Track 20. No phase or release gate is closed.

## Evidence

- Base: `132fb4d525c2bd7b110fc11548c8d1b5fdf9a648`.
- Original policy SHA-256: `17a514bd7c1b724632c841ea1a4ecd0113eb24d638588f92810db5207d0f76a7`.
- Adopted candidate SHA-256: `78f4231ecf70c8372842ff85666421cda3a65421a8c0db37a1f5260b76d23447`.
- cargo-deny 0.20.2 positive licence-only trial: exit 0, `licenses ok`
  on stdout; receipt SHA-256 `beeb1fb3c80e07465469d01d081c10b20a46216517da04317a9fa76925b6c6f0`.
- Wrong package, wrong version (2.0.3), and wrong licence (MIT) candidate trials:
  each exit 4, retaining the CC0 rejection. Receipt SHA-256
  `18219d11dad02b516d65e348c1576e3bda15864176122dde0bf98ee180ee4063`.
- Retained original command, tool, working-directory, lock and raw-log hashes:
  `/Users/doughnut/Documents/careops-sim/.artifacts/c1-policy-trial/`.
  Negative per-command timestamps were not captured. Unused global allowlist
  warnings remain recorded. The candidate was tested against the C1 standalone
  IO package and its locked both-feature graph; these are not whole-repository
  licence or advisory passes.

## Scope and remaining gates

Arrow's WASM transitive graph introduces this exact package; this exception does
not permit a different version or a global CC0 allowance. It does not waive
advisories, sources, MSRV, patent/trademark restrictions or PR #195's separate
security gate. Original trial logs remain immutable; adopting identical bytes
requires no fabricated repeat pass.

C1 implementation qualification and clean serial integration are still pending.
This local commit is prepared separately from the active IO worker's checkout,
Cargo manifests, lockfile and protected policy input. It has not been published,
merged or pinned by the parent repository. Integration must verify policy bytes
and run the affected combined gates on the joined source head.
