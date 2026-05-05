# Citation and Archival

This page is the source-of-truth summary for how KairoECS should be cited and archived.

## Where this fits

- Start from `website/src/index.md` for navigation.
- Use `docs/community/adoption.md` for the user path into the project.
- Use `docs/trustworthy-simulation/replay-and-seeds.md` and `docs/trustworthy-simulation/verification-validation-uncertainty.md` for reproducibility context.
- The current pre-release citation target is `0.4.0-alpha.1` and the repository code URL is `https://github.com/edithatogo/kairos`.

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
  - `date-released`
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
  - `datePublished`
  - `programmingLanguage`
  - `license`
  - `codeRepository`
  - `developmentStatus`
- `.zenodo.json`
  - `title`
  - `upload_type`
  - `version`
  - `publication_date`
  - `access_right`
  - `description`
  - `creators`
  - `license`
  - `keywords`

## DOI and Zenodo path

The DOI path is:

1. Keep `.zenodo.json` checked in as the release metadata seed.
2. Use a Zenodo sandbox or draft deposition first.
3. Promote the first archived release, expected to be `0.4.0-alpha.1`, to a Zenodo DOI only after the release notes and archive record are complete.
4. Record the minted DOI in the release notes and in the archive record.

The current archive path is therefore `CITATION.cff` -> `.zenodo.json` -> Zenodo draft/deposition -> DOI release.

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
