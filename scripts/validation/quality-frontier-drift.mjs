#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const EXPECTED_CHECKS = [
  'Rust core quality',
  'CodeQL (javascript)',
  'gitleaks',
  'Reject CI skip directives',
  'code and repository health',
].sort();
const EXPECTED_ACTION_PUBLISHERS = [
  'anchore/*',
  'codecov/*',
  'gitleaks/*',
  'github/*',
  'julia-actions/*',
  'lycheeverse/*',
  'ossf/*',
  'pypa/*',
  'r-lib/*',
  'Swatinem/*',
  'taiki-e/*',
  'zizmorcore/*',
].sort();
const EXPECTED_RENOVATE_PRESET = 'github>edithatogo/renovate-config';
const REQUIRED_CONTEXT_FILES = ['AGENTS.md', 'SECURITY.md', 'CONTRIBUTING.md'];
const EVIDENCE_SOURCES = {
  canonical_context_files: 'working-tree:AGENTS.md,SECURITY.md,CONTRIBUTING.md',
  one_command_ci_recipe: 'working-tree:justfile',
  source_renovate_preset: 'working-tree:renovate.json',
  source_renovate_dashboard: 'working-tree:renovate.json',
  main_context_files_current: 'GitHub REST: repos/{owner}/{repo}/contents/{path}?ref={resolved_default_branch_sha}',
  renovate_dashboard_present: 'GitHub REST: repos/{owner}/{repo}/issues/136',
  ruleset_active_for_main: 'GitHub REST: repos/{owner}/{repo}/rulesets/{id}',
  required_stable_checks: 'GitHub REST: active main ruleset required_status_checks',
  ruleset_blocks_delete_and_force_update: 'GitHub REST: active main ruleset rules',
  owner_recovery_requires_pull_request: 'GitHub REST: active main ruleset bypass_actors',
  branch_protection_solo_maintainer: 'GitHub REST: repos/{owner}/{repo}/branches/{branch}/protection',
  actions_settings: 'GitHub REST: repos/{owner}/{repo}/actions/permissions*',
  private_vulnerability_reporting_enabled: 'GitHub REST: repos/{owner}/{repo}/private-vulnerability-reporting',
  main_renovate_preset_and_dashboard: 'GitHub REST: repos/{owner}/{repo}/contents/renovate.json?ref={resolved_default_branch_sha}',
  renovate_refresh_after_integration: 'GitHub REST: latest merged pull request, issues/136, and open pull requests by renovate[bot]',
  current_required_check_runs: 'GitHub REST: commits/{default_branch_sha}/check-runs',
  trusted_main_codecov_upload_and_status: 'GitHub REST: commits/{default_branch_sha}/check-runs and /statuses',
};

function stable(value) {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stable(value[key])]));
  }
  return value;
}

function same(left, right) {
  return JSON.stringify(stable(left)) === JSON.stringify(stable(right));
}

function isAfterIntegration(activityAt, integrationAt) {
  return Boolean(integrationAt && activityAt && activityAt > integrationAt);
}

function addCheck(checks, name, expected, observed, status = undefined, note = undefined) {
  checks.push({
    name,
    status: status ?? (same(expected, observed) ? 'pass' : 'drift'),
    expected,
    observed,
    evidenceSource: EVIDENCE_SOURCES[name] ?? 'local quality-frontier validator',
    ...(note ? { note } : {}),
  });
}

function checkStatus(hosted, key) {
  return hosted.availability?.[key] === false ? 'unavailable' : undefined;
}

function requiredRuleChecks(ruleset) {
  return (ruleset?.rules ?? [])
    .filter((rule) => rule.type === 'required_status_checks')
    .flatMap((rule) => rule.parameters?.required_status_checks ?? [])
    .map((check) => check.context)
    .sort();
}

function evaluateDrift({ source, hosted }) {
  const checks = [];
  const sourceDocs = Object.fromEntries(REQUIRED_CONTEXT_FILES.map((path) => [path, Boolean(source.contextFiles?.[path])]));
  addCheck(checks, 'canonical_context_files', Object.fromEntries(REQUIRED_CONTEXT_FILES.map((path) => [path, true])), sourceDocs);
  addCheck(checks, 'source_clean_checkout', true, !source.dirty, source.dirty ? 'pending' : undefined,
    source.dirty ? 'Commit the working-tree changes before treating source evidence as immutable.' : undefined);
  addCheck(checks, 'one_command_ci_recipe', true, Boolean(source.hasJustCiRecipe));
  addCheck(checks, 'source_renovate_preset', EXPECTED_RENOVATE_PRESET, source.renovatePreset ?? null);
  addCheck(checks, 'source_renovate_dashboard', true, Boolean(source.renovateDashboard));

  const sourceContextHashes = { ...source.contextFileHashes, justfile: source.justfileHash };
  const contextHashesStatus = checkStatus(hosted, 'mainContextFiles') ?? (same(sourceContextHashes, hosted.mainContextFiles)
    ? 'pass' : (hosted.integrationPending || source.dirty ? 'pending' : 'drift'));
  addCheck(checks, 'main_context_files_current', sourceContextHashes, hosted.mainContextFiles,
    contextHashesStatus, contextHashesStatus === 'pending'
      ? 'Canonical context and validation source are on the candidate branch and await integration.' : undefined);

  const ruleset = hosted.ruleset;
  addCheck(checks, 'ruleset_active_for_main', {
    name: 'main quality and security gates',
    target: 'branch',
    enforcement: 'active',
    include: ['refs/heads/main'],
  }, ruleset ? {
    name: ruleset.name,
    target: ruleset.target,
    enforcement: ruleset.enforcement,
    include: ruleset.conditions?.ref_name?.include ?? [],
  } : null, checkStatus(hosted, 'ruleset'));
  addCheck(checks, 'required_stable_checks', EXPECTED_CHECKS, requiredRuleChecks(ruleset), checkStatus(hosted, 'ruleset'));
  addCheck(checks, 'ruleset_blocks_delete_and_force_update', ['deletion', 'non_fast_forward'].sort(),
    (ruleset?.rules ?? []).map((rule) => rule.type).filter((type) => ['deletion', 'non_fast_forward'].includes(type)).sort(), checkStatus(hosted, 'ruleset'));
  addCheck(checks, 'owner_recovery_requires_pull_request', ['pull_request'],
    (ruleset?.bypass_actors ?? []).map((actor) => actor.bypass_mode).sort(), checkStatus(hosted, 'ruleset'));

  const protection = hosted.protection ?? {};
  addCheck(checks, 'branch_protection_solo_maintainer', {
    forcePushes: false,
    deletions: false,
    adminsEnforced: true,
    linearHistory: true,
    requiredApprovals: 0,
    codeOwnerReview: false,
  }, {
    forcePushes: protection.allow_force_pushes?.enabled ?? null,
    deletions: protection.allow_deletions?.enabled ?? null,
    adminsEnforced: protection.enforce_admins?.enabled ?? null,
    linearHistory: protection.required_linear_history?.enabled ?? null,
    requiredApprovals: protection.required_pull_request_reviews?.required_approving_review_count ?? null,
    codeOwnerReview: protection.required_pull_request_reviews?.require_code_owner_reviews ?? null,
  }, checkStatus(hosted, 'protection'));

  addCheck(checks, 'actions_settings', {
    enabled: true,
    allowedActions: 'selected',
    shaPinningRequired: true,
    defaultWorkflowPermissions: 'read',
    canApprovePullRequests: false,
    githubOwnedAllowed: true,
    verifiedAllowed: false,
    publishers: EXPECTED_ACTION_PUBLISHERS,
  }, {
    enabled: hosted.actions?.enabled ?? null,
    allowedActions: hosted.actions?.allowed_actions ?? null,
    shaPinningRequired: hosted.actions?.sha_pinning_required ?? null,
    defaultWorkflowPermissions: hosted.workflowPermissions?.default_workflow_permissions ?? null,
    canApprovePullRequests: hosted.workflowPermissions?.can_approve_pull_request_reviews ?? null,
    githubOwnedAllowed: hosted.selectedActions?.github_owned_allowed ?? null,
    verifiedAllowed: hosted.selectedActions?.verified_allowed ?? null,
    publishers: [...(hosted.selectedActions?.patterns_allowed ?? [])].sort(),
  }, ['actions', 'workflowPermissions', 'selectedActions'].some((key) => checkStatus(hosted, key) === 'unavailable')
    ? 'unavailable' : undefined);

  addCheck(checks, 'private_vulnerability_reporting_enabled', true, hosted.privateReporting ?? null,
    checkStatus(hosted, 'privateReporting'));
  addCheck(checks, 'main_renovate_preset_and_dashboard', {
    preset: EXPECTED_RENOVATE_PRESET,
    dashboard: true,
  }, hosted.mainRenovate ? {
    preset: hosted.mainRenovate.extends?.find((preset) => preset.startsWith('github>')) ?? null,
    dashboard: Boolean(hosted.mainRenovate.dependencyDashboard),
  } : null, checkStatus(hosted, 'mainRenovate') ?? (hosted.mainRenovateStatus === 'pending' ? 'pending' : undefined),
  hosted.mainRenovateStatus === 'pending' ? 'The shared preset is present on the candidate branch and awaits integration.' : undefined);

  const dashboardPresent = hosted.renovateDashboard?.state === 'open' && hosted.renovateDashboard?.author === 'renovate[bot]';
  addCheck(checks, 'renovate_dashboard_present', true, dashboardPresent,
    checkStatus(hosted, 'renovateActivity'));
  const renovateRefreshStatus = checkStatus(hosted, 'renovateActivity') ?? (hosted.integrationPending
    ? 'pending'
    : (hosted.renovateRefreshedAfterIntegration ? 'pass' : 'pending'));
  addCheck(checks, 'renovate_refresh_after_integration', {
    evidence: 'Dashboard refresh or first bot PR after default-branch integration',
    refreshed: true,
  }, {
    integrationAt: hosted.renovateIntegrationAt ?? null,
    integrationPullRequest: hosted.latestMergedPullRequest ?? null,
    dashboard: hosted.renovateDashboard ?? null,
    qualifyingBotEvidence: hosted.renovateRefreshEvidence ?? [],
    refreshed: Boolean(hosted.renovateRefreshedAfterIntegration),
  }, renovateRefreshStatus, renovateRefreshStatus === 'pending'
    ? 'Wait for the hosted Renovate app to refresh the Dashboard or create/update an update PR after integration.' : undefined);

  const currentMainChecks = (hosted.mainChecks ?? []).filter((check) => check.head_sha === hosted.defaultBranchCommit);
  const codecovChecks = currentMainChecks
    .filter((check) => /codecov/i.test(check.name ?? '') || /codecov/i.test(check.app?.slug ?? ''))
    .map((check) => ({ name: check.name, appSlug: check.app?.slug, status: check.status, conclusion: check.conclusion, headSha: check.head_sha, startedAt: check.started_at, url: check.details_url }));
  const codecovStatuses = (hosted.mainStatuses ?? [])
    .filter((status) => /codecov/i.test(status.context ?? ''))
    .map((status) => ({ context: status.context, state: status.state, sha: status.sha, createdAt: status.created_at, url: status.target_url }));
  const latestUpload = codecovChecks
    .filter((check) => check.name === 'Codecov OIDC upload' && check.appSlug === 'github-actions')
    .reduce((latest, check) => !latest || (check.startedAt ?? '') > (latest.startedAt ?? '') ? check : latest, null);
  const latestProjectStatus = codecovStatuses
    .filter((status) => status.context === 'codecov/project' && status.sha === hosted.defaultBranchCommit)
    .reduce((latest, status) => !latest || (status.createdAt ?? '') > (latest.createdAt ?? '') ? status : latest, null);
  const hasSuccessfulUpload = latestUpload?.status === 'completed' && latestUpload?.conclusion === 'success';
  const hasCommitStatus = latestProjectStatus?.state === 'success';
  const currentCheckContexts = new Map();
  for (const check of currentMainChecks) {
    const previous = currentCheckContexts.get(check.name);
    if (!previous || (check.started_at ?? '') > (previous.started_at ?? '')) currentCheckContexts.set(check.name, check);
  }
  const requiredChecks = EXPECTED_CHECKS.map((name) => {
    const check = currentCheckContexts.get(name);
    return { name, status: check?.status ?? 'missing', conclusion: check?.conclusion ?? null, headSha: check?.head_sha ?? null, url: check?.details_url ?? null };
  });
  const allRequiredChecksPassed = requiredChecks.every((check) => check.status === 'completed' && check.conclusion === 'success');
  const requiredCheckStatus = checkStatus(hosted, 'mainChecks') ?? (allRequiredChecksPassed
    ? 'pass'
    : (hosted.integrationPending || requiredChecks.some((check) => check.status === 'queued' || check.status === 'in_progress')
      ? 'pending' : 'drift'));
  addCheck(checks, 'current_required_check_runs', EXPECTED_CHECKS.map((name) => ({ name, status: 'completed', conclusion: 'success', headSha: hosted.defaultBranchCommit })),
    requiredChecks, requiredCheckStatus, hosted.integrationPending ? 'Current default-branch checks are recorded after the candidate is integrated.' : undefined);
  const codecovStillRunning = ['queued', 'in_progress'].includes(latestUpload?.status) || latestProjectStatus?.state === 'pending';
  const codecovStatus = checkStatus(hosted, 'mainChecks') ?? checkStatus(hosted, 'mainStatuses') ?? (hosted.integrationPending
    ? 'pending' : (hasSuccessfulUpload && hasCommitStatus ? 'pass' : (codecovStillRunning ? 'pending' : 'drift')));
  addCheck(checks, 'trusted_main_codecov_upload_and_status', {
    successfulUploadCheck: 'Codecov OIDC upload completed successfully on default branch SHA',
    successfulProjectStatus: 'codecov/project success on default branch SHA',
  }, {
    successfulUploadCheck: hasSuccessfulUpload,
    successfulProjectStatus: hasCommitStatus,
    latestUpload,
    latestProjectStatus,
    checks: codecovChecks,
    statuses: codecovStatuses,
  }, codecovStatus, hosted.integrationPending
    ? 'The trusted-main upload and commit status can only be verified after the candidate is integrated.'
    : undefined);

  const status = checks.some((check) => check.status === 'drift')
    ? 'drift'
    : checks.some((check) => check.status === 'unavailable')
      ? 'unavailable'
      : checks.some((check) => check.status === 'pending')
        ? 'pending'
        : 'pass';
  for (const check of checks) {
    check.observedAt = source.generatedAt;
    check.targetSha = check.name.startsWith('source_') || check.name === 'canonical_context_files' || check.name === 'one_command_ci_recipe'
      ? source.commit : hosted.defaultBranchCommit;
    if (check.status === 'drift') check.nextAction = 'Correct the observed source or hosted setting, then regenerate this receipt.';
    if (check.status === 'pending') check.nextAction = 'Complete the named integration/provider readback and regenerate this receipt.';
    if (check.status === 'unavailable') check.nextAction = 'Restore read-only access to the named evidence source and regenerate this receipt.';
  }
  return {
    schemaVersion: 1,
    generatedAt: source.generatedAt,
    repository: source.repository,
    source: { branch: source.branch, commit: source.commit, dirty: source.dirty },
    hosted: { defaultBranch: hosted.defaultBranch, defaultBranchCommit: hosted.defaultBranchCommit, renovateIntegrationAt: hosted.renovateIntegrationAt ?? null },
    status,
    checks,
  };
}

function run(command, args) {
  return execFileSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function ghJson(path) {
  return JSON.parse(run('gh', ['api', path]));
}

function ghJsonOptional(path) {
  try {
    return { ok: true, value: ghJson(path) };
  } catch (error) {
    const detail = `${error.message}\n${error.stderr?.toString?.() ?? ''}`;
    if (/HTTP 404|Not Found|status 404/i.test(detail)) return { ok: true, value: null };
    throw error;
  }
}

function ghJsonFiltered(path, expression) {
  return JSON.parse(run('gh', ['api', path, '--jq', expression]));
}

function ghJsonLinesPaginated(path, expression) {
  const output = run('gh', ['api', '--paginate', path, '--jq', expression]);
  return output ? output.split(/\r?\n/).map((line) => JSON.parse(line)) : [];
}

function sourceSnapshot(root) {
  const git = (args) => run('git', ['-C', root, ...args]);
  const renovate = JSON.parse(readFileSync(resolve(root, 'renovate.json'), 'utf8'));
  const justfile = readFileSync(resolve(root, 'justfile'), 'utf8');
  return {
    generatedAt: new Date().toISOString(),
    repository: 'edithatogo/kairos',
    branch: git(['branch', '--show-current']),
    upstreamBranch: (() => {
      try { return git(['rev-parse', '--abbrev-ref', '--symbolic-full-name', '@{upstream}']).replace(/^origin\//, ''); }
      catch { return null; }
    })(),
    commit: git(['rev-parse', 'HEAD']),
    dirty: Boolean(git(['status', '--porcelain'])),
    contextFiles: Object.fromEntries(REQUIRED_CONTEXT_FILES.map((path) => [path, existsSync(resolve(root, path))])),
    contextFileHashes: Object.fromEntries(REQUIRED_CONTEXT_FILES.map((path) => [path,
      existsSync(resolve(root, path)) ? run('git', ['-C', root, 'hash-object', path]) : null])),
    justfileHash: run('git', ['-C', root, 'hash-object', 'justfile']),
    hasJustCiRecipe: /^ci:\s/m.test(justfile),
    renovatePreset: renovate.extends?.find((preset) => preset.startsWith('github>')) ?? null,
    renovateDashboard: Boolean(renovate.dependencyDashboard),
  };
}

function getMainRenovate(repository, defaultBranchCommit) {
  try {
    const result = ghJsonOptional(`repos/${repository}/contents/renovate.json?ref=${encodeURIComponent(defaultBranchCommit)}`);
    if (!result.value) return { ok: true, value: null };
    const entry = result.value;
    return { ok: true, value: JSON.parse(Buffer.from(entry.content, 'base64').toString('utf8')) };
  } catch (error) {
    return { ok: false, error: error.message };
  }
}

function getMainFileHash(repository, defaultBranchCommit, path) {
  const result = ghJsonOptional(`repos/${repository}/contents/${path}?ref=${encodeURIComponent(defaultBranchCommit)}`);
  return result.value?.sha ?? null;
}

function collectHosted(repository, source) {
  const repo = ghJson(`repos/${repository}`);
  const defaultBranch = repo.default_branch;
  const mainCommit = ghJson(`repos/${repository}/commits/${encodeURIComponent(defaultBranch)}`);
  const defaultBranchCommit = mainCommit.sha;
  const rulesets = ghJson(`repos/${repository}/rulesets`);
  const matchingRuleset = rulesets.find((candidate) => candidate.name === 'main quality and security gates');
  const ruleset = matchingRuleset ? ghJson(`repos/${repository}/rulesets/${matchingRuleset.id}`) : null;
  const protectionResult = ghJsonOptional(`repos/${repository}/branches/${encodeURIComponent(defaultBranch)}/protection`);
  const protection = protectionResult.value;
  const actions = ghJson(`repos/${repository}/actions/permissions`);
  const workflowPermissions = ghJson(`repos/${repository}/actions/permissions/workflow`);
  const selectedActions = ghJson(`repos/${repository}/actions/permissions/selected-actions`);
  const privateReporting = ghJson(`repos/${repository}/private-vulnerability-reporting`);
  const mainRenovateResult = getMainRenovate(repository, defaultBranchCommit);
  const mainRenovate = mainRenovateResult.value ?? null;
  const checkNames = EXPECTED_CHECKS.map((name) => `.name == ${JSON.stringify(name)}`).join(' or ');
  const checkRunFilter = `.check_runs[] | select(.head_sha == ${JSON.stringify(defaultBranchCommit)} and (${checkNames} or (.name | ascii_downcase | contains("codecov")) or (.app.slug | ascii_downcase | contains("codecov")))) | {name, status, conclusion, head_sha, started_at, details_url, app: {slug: .app.slug}}`;
  const mainChecks = ghJsonLinesPaginated(`repos/${repository}/commits/${defaultBranchCommit}/check-runs?per_page=100`, checkRunFilter);
  const mainStatuses = ghJsonFiltered(`repos/${repository}/commits/${defaultBranchCommit}/statuses`,
    '[.[] | {context, state, sha, created_at, target_url}]');
  const openPulls = ghJsonLinesPaginated(`repos/${repository}/pulls?state=open&per_page=100`,
    '.[] | {number, headRef: .head.ref, headSha: .head.sha, author: .user.login, createdAt: .created_at, updatedAt: .updated_at, url: .html_url}');
  const renovateDashboardResult = ghJsonOptional(`repos/${repository}/issues/136`);
  const renovateDashboard = renovateDashboardResult.value;
  const mainContextFiles = Object.fromEntries([...REQUIRED_CONTEXT_FILES, 'justfile'].map((path) => [
    path,
    getMainFileHash(repository, defaultBranchCommit, path),
  ]));
  const renovateComments = ghJsonLinesPaginated(`repos/${repository}/issues/136/comments?per_page=100`,
    '.[] | {author: .user.login, createdAt: .created_at, url: .html_url}');
  const candidatePullOpen = openPulls.some((pull) => pull.headSha === source.commit || (source.upstreamBranch && pull.headRef === source.upstreamBranch));
  const mainRenovateStatus = mainRenovate?.extends?.includes(EXPECTED_RENOVATE_PRESET)
    ? 'pass'
    : (candidatePullOpen && source.renovatePreset === EXPECTED_RENOVATE_PRESET ? 'pending' : 'drift');
  const mergedPulls = ghJsonLinesPaginated(`repos/${repository}/pulls?state=closed&base=${encodeURIComponent(defaultBranch)}&sort=updated&direction=desc&per_page=100`,
    '.[] | select(.merged_at != null) | {number, mergedAt: .merged_at, url: .html_url}');
  const latestMergedPull = mergedPulls.reduce((latest, pull) => !latest || pull.mergedAt > latest.mergedAt ? pull : latest, null);
  const integrationAt = latestMergedPull?.mergedAt ?? null;
  const renovateRefreshEvidence = integrationAt ? [
    ...openPulls.filter((pull) => pull.author === 'renovate[bot]' && isAfterIntegration(pull.createdAt, integrationAt))
      .map((pull) => ({ kind: 'pull_request', number: pull.number, createdAt: pull.createdAt, url: pull.url })),
    ...renovateComments.filter((comment) => comment.author === 'renovate[bot]' && isAfterIntegration(comment.createdAt, integrationAt))
      .map((comment) => ({ kind: 'dashboard_comment', createdAt: comment.createdAt, url: comment.url })),
  ] : [];
  return {
    defaultBranch,
    defaultBranchCommit,
    ruleset,
    protection,
    actions,
    workflowPermissions,
    selectedActions,
    privateReporting: typeof privateReporting === 'boolean' ? privateReporting : privateReporting.enabled,
    mainRenovate,
    mainRenovateStatus,
    renovateDashboard: renovateDashboard ? { state: renovateDashboard.state, author: renovateDashboard.user?.login, url: renovateDashboard.html_url } : null,
    latestMergedPullRequest: latestMergedPull,
    renovateIntegrationAt: integrationAt,
    renovateRefreshEvidence,
    renovateRefreshedAfterIntegration: renovateRefreshEvidence.length > 0,
    mainContextFiles,
    availability: {
      ruleset: true,
      protection: protectionResult.ok,
      actions: true,
      workflowPermissions: true,
      selectedActions: true,
      privateReporting: true,
      mainRenovate: mainRenovateResult.ok,
      mainContextFiles: true,
      renovateActivity: true,
      mainChecks: Array.isArray(mainChecks),
      mainStatuses: Array.isArray(mainStatuses),
    },
    mainChecks,
    mainStatuses,
    renovateOpenPullRequestCount: openPulls.filter((pull) => pull.author === 'renovate[bot]').length,
    integrationPending: candidatePullOpen,
  };
}

function main() {
  let root;
  let source;
  let receipt;
  try {
    root = run('git', ['rev-parse', '--show-toplevel']);
    source = sourceSnapshot(root);
    const hosted = collectHosted(source.repository, source);
    receipt = evaluateDrift({ source, hosted });
  } catch (error) {
    receipt = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      repository: source?.repository ?? 'edithatogo/kairos',
      source: source ? { branch: source.branch, commit: source.commit, dirty: source.dirty } : null,
      hosted: null,
      status: 'unavailable',
      checks: [{ name: 'hosted_readback', status: 'unavailable', expected: 'authenticated read-only GitHub API access', observed: error.message }],
    };
  }
  const outputIndex = process.argv.indexOf('--output');
  if (outputIndex >= 0) {
    if (!process.argv[outputIndex + 1]) throw new Error('--output needs a path');
    const outputPath = resolve(root, process.argv[outputIndex + 1]);
    mkdirSync(dirname(outputPath), { recursive: true });
    writeFileSync(outputPath, `${JSON.stringify(receipt, null, 2)}\n`);
    process.stdout.write(`${outputPath}\n`);
  } else {
    process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
  }
  if (receipt.status === 'drift') process.exitCode = 1;
  if (receipt.status === 'pending' || receipt.status === 'unavailable') process.exitCode = 2;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) main();

export { evaluateDrift, isAfterIntegration };
