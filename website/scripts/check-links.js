const fs = require("fs");
const path = require("path");

const siteRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(siteRoot, "..");
const manifestPath = path.join(siteRoot, "docs-link-manifest.json");

function existsRelative(relativePath) {
  return fs.existsSync(path.join(repoRoot, relativePath));
}

function stripFragment(link) {
  return link.split("#")[0].split("?")[0];
}

function isExternal(link) {
  return /^[a-z][a-z0-9+.-]*:/i.test(link) || link.startsWith("#");
}

function checkMarkdownLinks(sourceFile) {
  const absoluteSource = path.join(repoRoot, sourceFile);
  const source = fs.readFileSync(absoluteSource, "utf8");
  const sourceDir = path.dirname(absoluteSource);
  const failures = [];
  const linkPattern = /\[[^\]]+\]\(([^)]+)\)/g;
  let match;

  while ((match = linkPattern.exec(source)) !== null) {
    const rawLink = match[1].trim();
    const localLink = stripFragment(rawLink);
    if (!localLink || isExternal(localLink)) {
      continue;
    }

    const target = path.resolve(sourceDir, localLink);
    if (!target.startsWith(repoRoot) || !fs.existsSync(target)) {
      failures.push(`${sourceFile}: missing link target ${rawLink}`);
    }
  }

  return failures;
}

function checkNavigationLinks(manifest) {
  const failures = [];
  for (const section of manifest.navigationSections || []) {
    if (!section.title || !Array.isArray(section.links) || section.links.length === 0) {
      failures.push(`manifest: navigation section is incomplete: ${JSON.stringify(section)}`);
      continue;
    }
    for (const link of section.links) {
      if (!link.label || !link.path) {
        failures.push(`manifest: navigation link is incomplete in ${section.title}`);
        continue;
      }
      if (!existsRelative(link.path)) {
        failures.push(`manifest: navigation target missing ${link.path}`);
      }
    }
  }
  return failures;
}

function main() {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const failures = [];

  for (const requiredPath of manifest.requiredPaths) {
    if (!existsRelative(requiredPath)) {
      failures.push(`manifest: missing required path ${requiredPath}`);
    }
  }

  for (const sourceFile of manifest.siteSources) {
    if (!existsRelative(sourceFile)) {
      failures.push(`manifest: missing source ${sourceFile}`);
      continue;
    }
    failures.push(...checkMarkdownLinks(sourceFile));
  }

  failures.push(...checkNavigationLinks(manifest));

  if (failures.length > 0) {
    process.stderr.write(`${failures.join("\n")}\n`);
    process.exit(1);
  }

  process.stdout.write(
    `Checked ${manifest.requiredPaths.length} required paths, ${manifest.siteSources.length} markdown sources, and ${(manifest.navigationSections || []).length} navigation sections.\n`
  );
}

if (require.main === module) {
  main();
}
