# Authenticated Syft installation

This installer acquires Anchore Syft **1.54.0** only after verifying the
release's signed checksum document. It supports native Darwin arm64 and Linux
amd64 hosts, and writes a private receipt alongside the installed executable.
It does not run a vulnerability scan, publish an artifact, or modify a workflow.

## Pinned release identity

| Property | Required value |
|---|---|
| Repository and tag | `anchore/syft`, `v1.54.0` |
| Source commit | `cc326e45a6213360266dda4b30cc68095946d676` |
| Certificate identity | `https://github.com/anchore/syft/.github/workflows/release.yaml@refs/heads/main` |
| OIDC issuer | `https://token.actions.githubusercontent.com` |
| Sigstore CLI policy | `verify github`, exact identity, commit, repository, and `refs/heads/main` |

Sigstore **4.5.0**'s GitHub verifier hardcodes the GitHub Actions OIDC issuer and
does not accept a `--cert-oidc-issuer` option. The installer makes one
`sigstore verify github` invocation against the checksum text, with the
certificate identity, source commit, repository, and ref pinned above. Its two
verifier requirement files are byte-for-byte retained, hash-locked closures:

- Linux: `e8c2913539b2dc4260ef8611e1f21daa56efbdf8b55199c34882808aecd6acea`
- Darwin: `bc22323572381258237ff65529b55f37387a3bdfddf1ccf82305d41443ddacf2`

Each pins Sigstore 4.5.0 and pypi-attestations 0.0.30 and requires wheel hashes.
The installer first sets `PIP_CONFIG_FILE` to the null device, uses pip's
`--isolated` mode, audits the effective pip configuration, and then performs
the hash-required install. It invokes Python by absolute path and gives child
processes a fixed `/usr/bin:/bin` PATH. Ambient proxy, token, pip-index, and CA
override settings are not passed to children. HTTPS uses Python's default
verified TLS context; the receipt records the runtime's default CA paths.

The downloader accepts only the exact pinned GitHub release URLs, HTTPS on port
443, and bounded redirects between `github.com` and
`release-assets.githubusercontent.com`. It rejects unexpected media types,
non-200 responses, inconsistent or oversized lengths, stalled transfers, and
streaming overflow. The checksum text and Sigstore bundle must also match their
retained SHA-256 values before verifier installation or verification.

Only after signature verification does the installer parse the authenticated
checksum row and compare it with the retained asset digest. It downloads no
other release assets. The archive is checked before extraction and must contain
exactly four regular files: `CHANGELOG.md`, `LICENSE`, `README.md`, and `syft`.
Path traversal, duplicate canonical names, links, special files, extra or
missing entries, oversized members, and excessive expanded size are rejected. A
bounded gzip/tar pre-scan caps decompressed bytes and PAX metadata/header sizes;
archive parsing and extraction run in a timed child process group. The installed
executable is then checked with `syft version -o json` against
the pinned version, commit, application, and detected platform.

## Supported targets and qualification

| Target | Archive SHA-256 | Binary SHA-256 | Evidence status |
|---|---|---|---|
| Darwin arm64 | `7e0bdad94c569fc6d5785c9a657bbae3d4c4e140ccb5eace3d0b5b6bc2b6dbcf` | `835607cdfbdbfc59335b0beadeefc47aa6aab7d3b403c11cfa65627d92a27f61` | Binary pin retained; a fresh receipt is required for this installer source revision. |
| Linux amd64 | `54a87372498168b2d033e876fd41fa4e8035b872699e525a57046e1f2f09c860` | `d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92` | Binary pin independently read back from native run 37327710820; a fresh receipt is required for this installer source revision. |

Other operating systems and architectures fail closed. Both target pins are
bound to exact archive and binary bytes. The Linux lock is an exact retained
verifier dependency closure. The native receipts below were produced from the
previous installer source hash; changing the pins changes that source hash, so
fresh receipts on native Darwin arm64 and Linux amd64 are required before this
revision has current-source installer evidence.

## Native Darwin qualification record

The fresh native run completed successfully with all nine installer commands
exiting 0. It installed the byte-exact, hash-locked Sigstore 4.5.0 verifier,
verified the signed checksum document with the pinned GitHub identity, source
commit, repository, ref, and issuer policy, downloaded and extracted the signed
Darwin asset, and ran `syft version -o json`. The probe reported Syft 1.54.0,
commit `cc326e45a6213360266dda4b30cc68095946d676`, and `darwin/arm64`. The
observed archive and binary hashes matched the retained pins in the table.

The receipt and all nine bounded command logs were retained under
`.artifacts/verified-syft-installer/native-darwin-arm64/`. Receipt SHA-256:
`d7e878a4d484c2d4f4be23ea45b5f0fcfc3a5c7496f477c898b6c9f7fffe7681`. The
receipt records Python and executable hashes, input hashes, command arguments,
exit statuses, log lengths and hashes, and output hashes. It records the
earlier installer source revision and is not a fresh receipt for this pin
update. It does not represent a Syft scan.

## Native Linux qualification and raw-byte readback

The native Linux run completed all nine installer commands successfully on
`ubuntu-24.04` with Python 3.14.8. Run `37327710820` was a manual dispatch on
`main` at `ca2c42eb55facd166f5e30e133e7fb159511f522`; qualification job
`111822584000` uploaded artifact `11352233766` named
`syft-linux-native-qualification-37327710820-1`. The retained receipt SHA-256
is `79ec2da36d66b5466feaeb4cd02303abb16050deaa6a69d79eaa31cf380aaf28` and
records installer source hash
`05acf566b35cb1b4358bed4740e95b40c15c8148e90f251b47a4a60498f2a41b`. The
receipt and logs report Syft 1.54.0, commit
`cc326e45a6213360266dda4b30cc68095946d676`, and `linux/amd64`.

The hosted evidence artifact contains the receipt, validation report, and nine
logs, but not the runtime release archive or executable. A separate bounded
readback downloaded the exact pinned release TAR and used the installer's safe
extractor without executing the binary. The TAR was 29,217,540 bytes with SHA-256
`54a87372498168b2d033e876fd41fa4e8035b872699e525a57046e1f2f09c860`; its four
regular members were `CHANGELOG.md`, `LICENSE`, `README.md`, and `syft`. The
extracted Linux executable was 87,204,002 bytes with SHA-256
`d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92`, matching
the hosted receipt and validation report. These exact raw bytes establish the
Linux binary pin, but the run used the previous installer source hash; a fresh
native receipt must validate the updated source after merge.

## Use and retained evidence

Run with Python **3.14.8** on a supported native host and choose a new output
directory whose parent already exists:

```sh
python3 scripts/supply_chain/install_verified_syft.py \
  --output-dir .artifacts/verified-syft-installer/run-name
```

The installer rejects an existing destination or symlinked output ancestry. A
successful run leaves `bin/syft`, bounded command logs, downloaded release
inputs, and `evidence/receipt.json` under that new private directory. The
receipt records the authenticated inputs, asset and executable digests,
verifier lock hash, installer-source hash, Python toolchain and executable
hash, TLS default CA paths, exact child argument vectors, exit statuses, log
hashes, output byte counts, and the qualification limitation. It contains no
child environment values.

Tests use injected HTTP transport, verifier and scanner-command results. A
mocked success exercises ordering and receipt construction only; it is not
cryptographic verification or host qualification. The local unit tests do not
perform network access, dependency installation, real Sigstore verification,
or a Syft probe. The native runs above provide installer-path evidence for the
previous source revision only. No actual Syft scan, release, or publication is
represented here.
