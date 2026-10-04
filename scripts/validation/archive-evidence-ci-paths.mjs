// Exact evidence-only inputs covered by the archive regression workflow.
// Producer, workflow, classifier and unknown paths retain native verification.
const ARCHIVE_EVIDENCE_PATHS = new Set([
  "packaging/scripts/acquire_package_archive_bundle.py",
  "tests/test_package_archive_acquisition.py",
  "packaging/scripts/build_archive_supply_chain.py",
  "tests/test_archive_supply_chain.py",
  "tests/test_archive_supply_chain_ci.py",
  "scripts/archive-supply-chain-test-tools.in",
  "scripts/archive-supply-chain-test-tools.lock"
]);

export function isArchiveEvidencePath(path) {
  return typeof path === "string" && ARCHIVE_EVIDENCE_PATHS.has(path);
}
