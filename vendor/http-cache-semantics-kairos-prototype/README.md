# CareOps local npm override prototype

Private non-published experiment; not an official upstream release.

Source: official http-cache-semantics 4.2.0 registry tarball, SHA-512 verified. Original BSD-2-Clause LICENSE is preserved byte-for-byte.
Patch provenance: kornelski/http-cache-semantics PR #58 commit 14a8c2ad51740dc39bf3e8f1a11c845a5003f217; hellonewday/http-cache-semantics PR #1 commit 101a9e9a5b9aba5750a74b8f659c5646e90a962f.
Composed index.js SHA-256: ed6c1faabbe21f7bfef09ce258392cf181678149237a67ce492308a46ca6620c. Do not publish or describe as upstream 4.2.1.

Local hardening: split Connection header tokens on literal commas, then trim each token independently, avoiding repeated whitespace scans (CodeQL alert 490). Historical qualification receipts cover the earlier source; this amended payload requires fresh regression and hosted audit evidence.
