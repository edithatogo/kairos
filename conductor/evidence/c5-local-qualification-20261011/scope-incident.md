# Preserved integration scope failure

During integration of worker commits c23deab and b826aac, coordinator claim reserved only c5_workers.rs. The worker commits also contained five local harness/receipt files under .artifacts/c5-workers. The precommit scope check correctly failed for .artifacts/c5-workers/README.md, but the shell sequence lacked fail-fast and proceeded to commit c3ea398. This is a coordinator process failure, not a passing scope check. No source outside the commissioned C5 task changed.

Correction: fresh explicit reservation covers the affected artifacts, .gitignore and this evidence. Preserve worker receipt here; remove local standalone harness files from the tracked tree, retain originals in the worker worktree. Ignore C5-generated artifacts going forward. All subsequent integration sequences use set -e and inspect changed paths before mutation. Passing native tests do not retroactively repair this failed gate.
