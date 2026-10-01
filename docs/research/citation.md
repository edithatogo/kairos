# Citation and Archival

This page is the source-of-truth summary for how KairoECS should be cited and archived.

## Where this fits

- Start from `website/src/index.md` for navigation.
- Use `docs/community/adoption.md` for the user path into the project.
- Use `docs/trustworthy-simulation/replay-and-seeds.md` and `docs/trustworthy-simulation/verification-validation-uncertainty.md` for reproducibility context.
- The planned, unreleased metadata version is `0.4.0-alpha.1` and the repository code URL is `https://github.com/edithatogo/kairos`.

## Citation metadata

The checked-in citation metadata is split across three files:

- `CITATION.cff`
- `codemeta.json`
- `.zenodo.json`

The required fields are:

- `CITATION.cff`
  - `cff-version`
  - `message`
  - `title`
  - `version`
  - `type`
  - `authors`
  - `abstract`
  - `keywords`
  - `license`
  - `repository-code`
- `codemeta.json`
  - `@context`
  - `@type`
  - `name`
  - `description`
  - `version`
  - `programmingLanguage`
  - `license`
  - `codeRepository`
  - `developmentStatus`
- `.zenodo.json`
  - `title`
  - `upload_type`
  - `version`
  - `access_right`
  - `description`
  - `creators`
  - `license`
  - `keywords`

## DOI and Zenodo path

The first concrete archive target is the `0.4.0-alpha.1` pre-release. It is
not yet DOI-minted; until a Zenodo sandbox or draft deposition exists, release
notes must use this exact status instead of a placeholder DOI:

```text
Archive status: pre-release metadata seed, not yet DOI-minted
Release target: 0.4.0-alpha.1
Repository code: https://github.com/edithatogo/kairos
DOI/Zenodo link: none yet; add the Zenodo draft or minted DOI URL before any public release write
```

The DOI path is:

1. Keep `.zenodo.json` checked in as the release metadata seed.
2. Use a Zenodo sandbox or draft deposition first.
3. Promote the first archived release, expected to be `0.4.0-alpha.1`, to a Zenodo DOI only after the release notes and archive record are complete.
4. Record the minted DOI in the release notes and in the archive record.

The current archive path is therefore `CITATION.cff` -> `.zenodo.json` ->
Zenodo draft/deposition -> DOI release. The repository does not currently carry
a minted DOI; do not add a fake DOI, `TBD` DOI, or generic `10.xxxx` placeholder.

## Local validation

Run the Track 19 validator before editing release notes or archive metadata:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1
```

The validator checks that `CITATION.cff`, `codemeta.json`, `.zenodo.json`,
`paper/paper.md`, `paper/paper.bib`, and this guide agree on planned version, lifecycle, dates when released,
repository URL, title, license, and the current non-minted archive status.

## Archive notes

Each archived release must record:

- release version
- archive status, such as draft, sandbox, or DOI-minted
- DOI or draft deposition link
- source archive location
- reproducibility instructions
- any citation or metadata changes since the previous release
- the repository code URL used for the release metadata

## Release-note requirements

Release notes must include:

- the release version
- the citation files used for the release
- whether the release is archived, draft, or DOI-minted
- the Zenodo or archive link
- reproducibility instructions
- any version, author, or repository-code updates
- the release metadata version, if it differs from the source code tag

If a release is not yet archived, the release note must say that explicitly.

## Unreleased metadata and licensing (#91)

`0.4.0-alpha.1` is a planned metadata seed, not an existing Git tag or GitHub
Release. The 2026-10-01 GitHub readback returned no tags and no releases; its
commands and source revision are recorded in `release-metadata-status.json`.
Publication dates are omitted until a named release exists. Cite the full commit
SHA used for an experiment while working from unreleased source.

The root LICENSE and LICENSE.md already grant Apache-2.0 OR MIT; Rust crate
metadata inherits that same policy. This correction changes metadata, not rights.
Binding packages retain their declared license subsets (Python, TypeScript and
C# Apache-2.0; R MIT) under the existing root dual-license grant.

Before switching the lifecycle record to `released`, record the full source SHA,
version, license, tag and release URLs, artifact hashes/locations, and validation
receipts; add the same actual publication date to CFF, CodeMeta and Zenodo.
Run the local validator, then read back the tag, release and artifacts from GitHub.
Local consistency cannot prove release publication or external acceptance. Issue
#91 remains open until that reviewed exact-release evidence exists. No tag,
release, DOI or provider submission is created by this metadata correction.
