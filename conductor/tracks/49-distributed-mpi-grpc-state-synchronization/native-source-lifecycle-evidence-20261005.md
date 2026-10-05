# Track49 PRRTE/PMIx source and lifecycle evidence — 2026-10-05

Disposition: verified bounded source/licence provenance; native support-policy gate HOLD.
Base baef8780c5876d6be609b5a2a880e0a91cef21f8; isolated Track49 worktree.
No provider installation, rebuild, production adoption, release waiver, source/API,
manifest, global status, parallel path or parent pin edits.

## Source inputs independently retrieved

Read-only HTTPS retrievals used30-second per-request timeouts. All9 initial
retrievals succeeded; exact URL/final URL/UTC/status/bytes/SHA256 retained below.
No downloaded source or patch was executed. Selected tar members were retained
without extracting arbitrary paths; absolute/traversal names reject.

| Input | Verified SHA256 |
| --- | --- |
| PRRTE4.1.0 upstream release archive |285ad62b670075708b9fcfe14c54baa599733bc274d10502a82e8eebba0b7c70|
| PMIx6.1.0 upstream release archive |bb9021c8e100a376f5070ecca727f83a29b5f652dfe381793b88daa79a3b98a2|
| PRRTE378c61c backport bytes |64faa1acb89eddea096307a2658b11ccdaf85dc8c870fed4b3f8670329706a4f|

[PRRTE archive](https://github.com/openpmix/prrte/releases/download/v4.1.0/prrte-4.1.0.tar.bz2)
and [PMIx archive](https://github.com/openpmix/openpmix/releases/download/v6.1.0/pmix-6.1.0.tar.bz2)
match installed formula/SPDX source checksum declarations. The
[backport](https://github.com/openpmix/prrte/commit/378c61c1d8eff9858a7774c869fbd332c48711a8)
bytes match the installed formula declaration. It guards missing hwloc object types;
a backport does not prove a security-maintenance commitment.

Both archive LICENSE files match installed LICENSE byte-for-byte. Installed formula
licence declarations are BSD-3-Clause-Open-MPI. This is provenance, not final licence
policy acceptance. The retained PMIx compression plugin source files are reference
source only; no binary/source-reproduction assertion follows.

## Actual current support-policy gate

Pinned current repository policies independently retrieved and hash-checked:

- [PRRTE at8c6b70a](https://github.com/openpmix/prrte/blob/8c6b70a5a7b63fd2c7f55323ec96c4c7962a30d1/SECURITY.md), SHA256 b0d983174cd73a1e7304066140944a3762e529a49f7673676803e7f7350c11e8.
- [PMIx at436b171](https://github.com/openpmix/openpmix/blob/436b17118000e4afd8acf6329af7db03cdf10532/SECURITY.md), SHA25617e87ab1e56c68f142a72b90945ae2c39eefe60ed749a224349a6427667b141f.

These repository policies, not merely latest development documentation, designate
PRRTE5.0.x and PMIx7.0.x as the future supported series and earlier series as EOL
without security backports. Current upstream policy excludes the installed series
from promised security backports; successor series are presently prerelease.
Supported production-provider selection is unresolved.

Observed release API lists show PRRTEv5.0.0rc1 and PMIxv7.0.0rc1 as prereleases,
published2026-09-24, alongside installed stable4.1.0/6.1.0. Maintenance releases
3.0.14/5.0.11 are also present. Neither date sorting nor GitHub latest-release
selection establishes which candidate meets the current support policy. This is
not authority to downgrade or silently adopt a prerelease.
[PRRTE releases](https://github.com/openpmix/prrte/releases),
[PMIx releases](https://github.com/openpmix/openpmix/releases).

Public repository advisory listings were inspected separately; their absence of
published advisories is not native vulnerability clearance or proof of safety.
No known exploitable flaw was established by this bounded audit. Historic PMIx
security warnings and all relevant native dependencies still require exact-version
advisory assessment before production acceptance; Cargo audit does not cover them.

## Packaging and reproduction limitations

Both Homebrew formulas refer to Patches/libtool/configure-big_sur.diff. Exact
libtool patch bytes and complete patched-tree reconstruction remain unverified.
PRRTE's extra backport is verified as source bytes only, not its application to
that installed bottle. Installed bottle archive authenticity and reproducible
binary-to-patched-source mapping remain unproven. Metadata receipts, current linked
hashes and runtime traces retain their own narrower evidence scope.

The previous actual MPI2/4 traces remain accepted for local collective/provider
observations with hwloc2.15.0. This policy finding does not rewrite them as failures,
but prevents promoting that provider stack to a supported production dependency.
No native support exception, advisory dismissal or release decision is granted.

## Independent review and next required action

Qualified distributed/security reviewer independently checked current repository
policies/release evidence, installed formulas/licences, backport and packaging gaps.
Exact retained artifacts/record require final review before integration.

Research-only next leaf: bind PRRTE5/PMIx7 release-candidate assets/checksums and
OpenMPI compatibility requirements; inspect exact Homebrew libtool inputs and
candidate launch/authentication behavior. Do not install/adopt until reviewed
provider/support disposition. All supported platforms, dependency-source ownership,
complete backend/API/schema/golden freeze and live distributed parity/cut/recovery
remain open. Pending material wire amendment approval remains separate.

## Retained evidence bindings

- `artifacts/track49-native-lifecycle-20261005/retrievals.json` SHA256 `bdb379a7db5a6aaf154843921b8be9d2dafdcbd50a4d853dc05a790932404c05`.
- `artifacts/track49-native-lifecycle-20261005/source-members.json` SHA256 `e41cd97c3f042fae5d13961313d464791df3c9d001db83023e3a88d4709838e0`.
- `artifacts/track49-native-lifecycle-20261005/policy-pins.json` SHA256 `a3d1977cbf6c6ad9057baa967adf6ac99dd3fc7f205558cfdb0a89b361f7a977`.
- `artifacts/track49-native-lifecycle-20261005/prrte-releases.json` SHA256 `c6717df840ba3f1f49e6ac9adf828794d27e1396bcfb6c917c9cc9dee923f85b`.
- `artifacts/track49-native-lifecycle-20261005/openpmix-releases.json` SHA256 `e8d6df0f97e5e96ad1afe8f1a32b75b674465cccfd4bb047312745223e292a90`.
- `artifacts/track49-native-lifecycle-20261005/prrte-source.tar.bz2` SHA256 `285ad62b670075708b9fcfe14c54baa599733bc274d10502a82e8eebba0b7c70`.
- `artifacts/track49-native-lifecycle-20261005/pmix-source.tar.bz2` SHA256 `bb9021c8e100a376f5070ecca727f83a29b5f652dfe381793b88daa79a3b98a2`.
- `artifacts/track49-native-lifecycle-20261005/prrte-backport.patch` SHA256 `64faa1acb89eddea096307a2658b11ccdaf85dc8c870fed4b3f8670329706a4f`.
- `artifacts/track49-native-lifecycle-20261005/prrte-security-pinned.txt` SHA256 `b0d983174cd73a1e7304066140944a3762e529a49f7673676803e7f7350c11e8`.
- `artifacts/track49-native-lifecycle-20261005/openpmix-security-pinned.txt` SHA256 `17e87ab1e56c68f142a72b90945ae2c39eefe60ed749a224349a6427667b141f`.
- `artifacts/track49-native-lifecycle-20261005/prrte-source/LICENSE` SHA256 `f0f8076249a700084c141ec8c70b9b199dd05fd3a0e0e5191d130f24debd20af`.
- `artifacts/track49-native-lifecycle-20261005/pmix-source/LICENSE` SHA256 `1e417caee654e1ad14fe81f4d2e4e5449173f0057edb0e7f2f96a15889f38eb8`.
- `artifacts/track49-native-lifecycle-20261005/prrte-installed/prrte.rb` SHA256 `69484a6db2a3baeee0a44899d08693eef43a75b1c90309c0403e15c07414e914`.
- `artifacts/track49-native-lifecycle-20261005/pmix-installed/pmix.rb` SHA256 `1c1deb190b30a835e0cc70ab420c34156c049dc59c0494bf6618b9b78755536d`.
