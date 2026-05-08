const fs = require("fs");
const path = require("path");

const siteRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(siteRoot, "..");
const manifestPath = path.join(siteRoot, "docs-link-manifest.json");
const docsRoots = ["docs", "bindings", "examples/docs", "website/src"];

function existsRelative(relativePath) {
  return fs.existsSync(path.join(repoRoot, relativePath));
}

function slugify(text) {
  const slug = text
    .toLowerCase()
    .replace(/`([^`]+)`/g, "$1")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "section";
}

function parseLocalLink(link) {
  const [rawTargetPath, rawFragment = ""] = link.split("#");
  const targetPath = rawTargetPath.split("?")[0];
  const fragment = rawFragment.split("?")[0];
  return {
    targetPath,
    fragment: decodeFragment(fragment),
  };
}

function isExternal(link) {
  return /^[a-z][a-z0-9+.-]*:/i.test(link);
}

function decodeFragment(fragment) {
  try {
    return fragment ? decodeURIComponent(fragment) : "";
  } catch (_) {
    return fragment;
  }
}

function isWithinRepo(target, root = repoRoot) {
  const relative = path.relative(root, target);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function collectMarkdownSources(relativeRoot) {
  const absoluteRoot = path.join(repoRoot, relativeRoot);
  const entries = [];

  if (!fs.existsSync(absoluteRoot)) {
    return entries;
  }

  const stack = [absoluteRoot];
  while (stack.length > 0) {
    const current = stack.pop();
    let dirEntries;
    try {
      dirEntries = fs.readdirSync(current, { withFileTypes: true });
    } catch (error) {
      if (error && (error.code === "EPERM" || error.code === "ENOENT")) {
        continue;
      }
      throw error;
    }
    for (const entry of dirEntries) {
      const absoluteEntry = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(absoluteEntry);
        continue;
      }
      if (entry.isFile() && entry.name.toLowerCase().endsWith(".md")) {
        entries.push(path.relative(repoRoot, absoluteEntry));
      }
    }
  }

  return entries;
}

function markdownAnchors(absolutePath) {
  const source = fs.readFileSync(absolutePath, "utf8");
  const anchors = new Set();

  for (const line of source.split(/\r?\n/)) {
    const headingMatch = line.match(/^(#{1,3})\s+(.+)$/);
    if (headingMatch) {
      anchors.add(slugify(headingMatch[2].trim()));
    }
  }

  return anchors;
}

function checkMarkdownLinks(sourceFile, root = repoRoot) {
  const absoluteSource = path.join(root, sourceFile);
  const source = fs.readFileSync(absoluteSource, "utf8");
  const sourceDir = path.dirname(absoluteSource);
  const failures = [];
  const linkPattern = /\[[^\]]+\]\(([^)]+)\)/g;
  let match;

  while ((match = linkPattern.exec(source)) !== null) {
    const rawLink = match[1].trim();
    if (isExternal(rawLink)) {
      continue;
    }

    const { targetPath, fragment } = parseLocalLink(rawLink);
    const target = targetPath ? path.resolve(sourceDir, targetPath) : absoluteSource;
    if (!isWithinRepo(target, root) || !fs.existsSync(target)) {
      failures.push(`${sourceFile}: missing link target ${rawLink}`);
      continue;
    }

    if (fragment && target.toLowerCase().endsWith(".md")) {
      const anchors = markdownAnchors(target);
      if (!anchors.has(fragment)) {
        failures.push(`${sourceFile}: missing anchor #${fragment} in ${path.relative(root, target)}`);
      }
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

function checkDocsTreeCoverage(manifest) {
  const failures = [];
  const seen = new Set();

  for (const root of docsRoots) {
    for (const sourceFile of collectMarkdownSources(root)) {
      if (seen.has(sourceFile)) {
        continue;
      }
      seen.add(sourceFile);
      failures.push(...checkMarkdownLinks(sourceFile));
    }
  }

  for (const sourceFile of manifest.siteSources || []) {
    if (!existsRelative(sourceFile)) {
      failures.push(`manifest: missing source ${sourceFile}`);
    }
  }

  return failures;
}

function runSelfTest() {
  const tempRoot = fs.mkdtempSync(path.join(require("os").tmpdir(), "kairo-doc-links-"));
  try {
    fs.mkdirSync(path.join(tempRoot, "docs"), { recursive: true });
    fs.writeFileSync(
      path.join(tempRoot, "docs", "target.md"),
      "# Target Page\n\n## Valid Anchor\n",
      "utf8"
    );
    fs.writeFileSync(
      path.join(tempRoot, "docs", "source.md"),
      "[ok](target.md#valid-anchor)\n[bad](target.md#missing-anchor)\n[same](#local-anchor)\n## Local Anchor\n",
      "utf8"
    );

    const sourceFile = "docs/source.md";
    const failures = checkMarkdownLinks(sourceFile, tempRoot);
    if (failures.length !== 1 || !failures[0].includes("missing-anchor")) {
      throw new Error(`anchor self-test failed: ${JSON.stringify(failures)}`);
    }
    process.stdout.write("Docs link anchor self-test passed.\n");
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
}

function main() {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const failures = [];

  const criticalPaths = [
    "docs/install.md",
    "docs/ffi/ffi-guide.md",
    "docs/arrow/schema-reference.md",
    "docs/tutorials/r-getting-started.md",
    "docs/tutorials/julia-getting-started.md",
    "docs/tutorials/csharp-getting-started.md",
    "docs/tutorials/go-getting-started.md",
  ];

  for (const criticalPath of criticalPaths) {
    if (!manifest.requiredPaths.includes(criticalPath)) {
      failures.push(`manifest: missing required path ${criticalPath}`);
    }
  }

  for (const requiredPath of manifest.requiredPaths) {
    if (!existsRelative(requiredPath)) {
      failures.push(`manifest: missing required path ${requiredPath}`);
    }
  }

  failures.push(...checkDocsTreeCoverage(manifest));
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
  if (process.argv.includes("--self-test")) {
    runSelfTest();
  } else {
    main();
  }
}
