import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ciSkipGuardContract, evaluateDrift, isAfterIntegration } from '../../scripts/validation/quality-frontier-drift.mjs';

assert.deepEqual(ciSkipGuardContract(readFileSync('.github/workflows/ci-skip-guard.yml', 'utf8')),
  { pullRequestTrigger: true, jobName: true });

assert.equal(isAfterIntegration('2026-09-29T00:00:01Z', '2026-09-29T00:00:00Z'), true);
assert.equal(isAfterIntegration('2026-09-29T00:00:00Z', '2026-09-29T00:00:00Z'), false);
assert.equal(isAfterIntegration('2026-09-28T23:59:59Z', '2026-09-29T00:00:00Z'), false);
assert.equal(isAfterIntegration('2026-09-29T00:00:01Z', null), false);

const source = {
  generatedAt: '2026-09-29T00:00:00.000Z',
  repository: 'edithatogo/kairos',
  branch: 'main',
  commit: 'abc123',
  dirty: false,
  ciSkipGuardHash: 'blob-ci-skip-guard',
  ciSkipGuardContract: { pullRequestTrigger: true, jobName: true },
  contextFiles: { 'AGENTS.md': true, 'SECURITY.md': true, 'CONTRIBUTING.md': true },
  contextFileHashes: { 'AGENTS.md': 'blob-agents', 'SECURITY.md': 'blob-security', 'CONTRIBUTING.md': 'blob-contributing' },
  justfileHash: 'blob-justfile',
  hasJustCiRecipe: true,
  renovatePreset: 'github>edithatogo/renovate-config',
  renovateDashboard: true,
};
const protection = {
  allow_force_pushes: { enabled: false },
  allow_deletions: { enabled: false },
  enforce_admins: { enabled: true },
  required_linear_history: { enabled: true },
  required_pull_request_reviews: { required_approving_review_count: 0, require_code_owner_reviews: false },
};
const requiredChecks = [
  'Rust core quality',
  'CodeQL (javascript)',
  'gitleaks',
  'Reject CI skip directives',
  'code and repository health',
];
const pushRequiredChecks = requiredChecks.filter((name) => name !== 'Reject CI skip directives');
const hosted = {
  defaultBranch: 'main',
  defaultBranchCommit: 'abc123',
  integrationPending: false,
  ruleset: {
    name: 'main quality and security gates',
    target: 'branch',
    enforcement: 'active',
    conditions: { ref_name: { include: ['refs/heads/main'] } },
    rules: [
      { type: 'deletion' },
      { type: 'non_fast_forward' },
      { type: 'required_status_checks', parameters: { required_status_checks: requiredChecks.map((context) => ({ context })) } },
    ],
    bypass_actors: [{ bypass_mode: 'pull_request' }],
  },
  protection,
  actions: { enabled: true, allowed_actions: 'selected', sha_pinning_required: true },
  workflowPermissions: { default_workflow_permissions: 'read', can_approve_pull_request_reviews: false },
  selectedActions: {
    github_owned_allowed: true,
    verified_allowed: false,
    patterns_allowed: ['anchore/*', 'codecov/*', 'gitleaks/*', 'github/*', 'julia-actions/*', 'lycheeverse/*', 'ossf/*', 'pypa/*', 'r-lib/*', 'Swatinem/*', 'taiki-e/*', 'zizmorcore/*'],
  },
  privateReporting: true,
  mainRenovate: { extends: ['github>edithatogo/renovate-config'], dependencyDashboard: true },
  mainRenovateStatus: 'pass',
  renovateDashboard: { state: 'open', author: 'renovate[bot]', url: 'https://github.com/edithatogo/kairos/issues/136' },
  renovateRefreshEvidence: [{ kind: 'pull_request', number: 157, createdAt: '2026-09-29T00:00:00Z' }],
  renovateRefreshedAfterIntegration: true,
  mainChecks: [
    ...pushRequiredChecks.map((name) => ({ name, status: 'completed', conclusion: 'success', head_sha: 'abc123', started_at: '2026-09-29T00:00:00Z' })),
    { name: 'Codecov OIDC upload', status: 'completed', conclusion: 'success', head_sha: 'abc123', started_at: '2026-09-29T00:00:00Z', app: { slug: 'github-actions' } },
  ],
  mainStatuses: [
    { context: 'codecov/project', state: 'success', sha: 'abc123', created_at: '2026-09-29T00:00:00Z' },
    { context: 'codecov/patch', state: 'failure', sha: 'abc123', created_at: '2026-09-29T00:00:00Z' },
  ],
  mainContextFiles: { 'AGENTS.md': 'blob-agents', 'SECURITY.md': 'blob-security', 'CONTRIBUTING.md': 'blob-contributing', justfile: 'blob-justfile' },
  mainCiSkipGuard: { sha: 'blob-ci-skip-guard', pullRequestTrigger: true, jobName: true },
  availability: {
    ruleset: true, protection: true, actions: true, workflowPermissions: true,
    selectedActions: true, privateReporting: true, mainRenovate: true, mainContextFiles: true,
    renovateActivity: true,
    mainChecks: true, mainStatuses: true, mainCiSkipGuard: true,
  },
};

const complete = evaluateDrift({ source, hosted });
assert.equal(complete.status, 'pass');
assert.ok(complete.checks.every((check) => check.status === 'pass'));

const pending = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    integrationPending: true,
    mainRenovate: { extends: ['config:recommended'], dependencyDashboard: true },
    mainRenovateStatus: 'pending',
    mainChecks: hosted.mainChecks.filter((check) => !/codecov/i.test(check.name)),
    mainStatuses: [],
    mainContextFiles: { 'AGENTS.md': null, 'SECURITY.md': null, 'CONTRIBUTING.md': null, justfile: null },
    mainCiSkipGuard: null,
    renovateDashboard: { state: 'open', author: 'renovate[bot]' },
    renovateRefreshedAfterIntegration: false,
  },
});
assert.equal(pending.status, 'pending');
assert.equal(pending.checks.find((check) => check.name === 'source_clean_checkout').status, 'pass');
assert.equal(pending.checks.find((check) => check.name === 'main_renovate_preset_and_dashboard').status, 'pending');
assert.equal(pending.checks.find((check) => check.name === 'trusted_main_codecov_upload_and_status').status, 'pending');

const drift = evaluateDrift({
  source,
  hosted: { ...hosted, selectedActions: { ...hosted.selectedActions, patterns_allowed: ['*'] } },
});
assert.equal(drift.status, 'drift');
assert.equal(drift.checks.find((check) => check.name === 'actions_settings').status, 'drift');

const missingSkipGuardRule = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    ruleset: {
      ...hosted.ruleset,
      rules: hosted.ruleset.rules.map((rule) => rule.type === 'required_status_checks'
        ? { ...rule, parameters: { required_status_checks: rule.parameters.required_status_checks.filter((check) => check.context !== 'Reject CI skip directives') } }
        : rule),
    },
  },
});
assert.equal(missingSkipGuardRule.checks.find((check) => check.name === 'required_stable_checks').status, 'drift');

const missingPushCheck = evaluateDrift({
  source,
  hosted: { ...hosted, mainChecks: hosted.mainChecks.filter((check) => check.name !== 'gitleaks') },
});
assert.equal(missingPushCheck.checks.find((check) => check.name === 'current_required_check_runs').status, 'drift');

const changedSkipGuard = evaluateDrift({
  source,
  hosted: { ...hosted, mainCiSkipGuard: { ...hosted.mainCiSkipGuard, sha: 'old-blob' } },
});
assert.equal(changedSkipGuard.checks.find((check) => check.name === 'main_ci_skip_guard_workflow_current').status, 'drift');

const invalidSkipGuardSource = evaluateDrift({
  source: { ...source, ciSkipGuardContract: { pullRequestTrigger: false, jobName: true } },
  hosted,
});
assert.equal(invalidSkipGuardSource.checks.find((check) => check.name === 'source_ci_skip_guard_contract').status, 'drift');

const unavailable = evaluateDrift({ source, hosted: { ...hosted, availability: { ...hosted.availability, ruleset: false } } });
assert.equal(unavailable.checks.find((check) => check.name === 'ruleset_active_for_main').status, 'unavailable');
assert.equal(unavailable.status, 'unavailable');

const wrongCodecov = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    mainChecks: hosted.mainChecks.filter((check) => check.name !== 'Codecov OIDC upload'),
    mainStatuses: [{ context: 'codecov/patch', state: 'success', sha: 'abc123' }],
  },
});
assert.equal(wrongCodecov.checks.find((check) => check.name === 'trusted_main_codecov_upload_and_status').status, 'drift');

const newerCodecovFailure = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    mainChecks: [
      ...hosted.mainChecks,
      { name: 'Codecov OIDC upload', status: 'completed', conclusion: 'failure', head_sha: 'abc123', started_at: '2026-09-29T00:01:00Z', app: { slug: 'github-actions' } },
    ],
    mainStatuses: [
      ...hosted.mainStatuses,
      { context: 'codecov/project', state: 'failure', sha: 'abc123', created_at: '2026-09-29T00:01:00Z' },
    ],
  },
});
assert.equal(newerCodecovFailure.checks.find((check) => check.name === 'trusted_main_codecov_upload_and_status').status, 'drift');

const newerCodecovPending = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    mainChecks: [
      ...hosted.mainChecks,
      { name: 'Codecov OIDC upload', status: 'in_progress', conclusion: null, head_sha: 'abc123', started_at: '2026-09-29T00:01:00Z', app: { slug: 'github-actions' } },
    ],
    mainStatuses: [
      ...hosted.mainStatuses,
      { context: 'codecov/project', state: 'pending', sha: 'abc123', created_at: '2026-09-29T00:01:00Z' },
    ],
  },
});
assert.equal(newerCodecovPending.checks.find((check) => check.name === 'trusted_main_codecov_upload_and_status').status, 'pending');

const spoofedCodecovCheck = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    mainChecks: hosted.mainChecks.map((check) => check.name === 'Codecov OIDC upload'
      ? { ...check, app: { slug: 'other-app' } } : check),
  },
});
assert.equal(spoofedCodecovCheck.checks.find((check) => check.name === 'trusted_main_codecov_upload_and_status').status, 'drift');

const staleContext = evaluateDrift({ source, hosted: { ...hosted, mainContextFiles: { ...hosted.mainContextFiles, 'AGENTS.md': 'old-blob' } } });
assert.equal(staleContext.checks.find((check) => check.name === 'main_context_files_current').status, 'drift');

const staleChecks = evaluateDrift({
  source,
  hosted: {
    ...hosted,
    mainChecks: hosted.mainChecks.map((check) => ({ ...check, head_sha: 'stale-sha' })),
  },
});
assert.equal(staleChecks.checks.find((check) => check.name === 'current_required_check_runs').status, 'drift');

const dirtySource = evaluateDrift({ source: { ...source, dirty: true }, hosted });
assert.equal(dirtySource.checks.find((check) => check.name === 'source_clean_checkout').status, 'pending');

console.log('quality frontier drift receipt checks passed (pass, pending, drift, unavailable)');
