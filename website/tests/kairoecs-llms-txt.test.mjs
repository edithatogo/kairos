import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { isPromotedIndexId } from "../src/plugins/kairoecs-llms-txt/generator.mjs";
import { entryToSimpleMarkdown } from "../src/plugins/kairoecs-llms-txt/entry-to-simple-markdown.mjs";
import { validateLlmsOptions } from "../src/plugins/kairoecs-llms-txt/index.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

function docsIds(directory, prefix = "") {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) return docsIds(full, `${prefix}${entry.name}/`);
    if (!/\.(md|mdx)$/.test(entry.name)) return [];
    return [`${prefix}${entry.name.replace(/\.(md|mdx)$/, "")}`];
  });
}

test("fixed index* matcher matches the pinned micromatch truth table and current collection", () => {
  const cases = new Map([
    ["index", true],
    ["index-extra", true],
    ["index/nested", false],
    ["Index", false],
    ["nested/index", false],
    ["index.foo", true],
  ]);
  for (const [id, expected] of cases) assert.equal(isPromotedIndexId(id), expected, id);
  const ids = docsIds(path.join(root, "website/src/content/docs")).sort();
  assert.ok(ids.includes("index"), "the docs collection includes index");
  assert.deepEqual(ids.filter(isPromotedIndexId), ["index"]);
});

test("options accept the configured shape and reject unsupported or malformed values", () => {
  assert.deepEqual(validateLlmsOptions({
    projectName: "KairoECS",
    description: "description",
    details: "details",
    optionalLinks: [{ label: "Repository", url: "https://github.com/edithatogo/kairos" }],
  }).optionalLinks, [{ label: "Repository", url: "https://github.com/edithatogo/kairos" }]);
  assert.deepEqual(validateLlmsOptions({}).optionalLinks, []);
  for (const key of ["customSets", "include", "exclude", "promote", "demote", "minify", "customSelectors", "pageSeparator", "rawContent", "unknownOption"]) {
    assert.throws(() => validateLlmsOptions({ [key]: [] }), new RegExp(key));
  }
  for (const options of [
    { projectName: 3 },
    { description: null },
    { details: false },
    { optionalLinks: {} },
    { optionalLinks: [{ label: "missing url" }] },
    { optionalLinks: [{ label: "ok", url: "https://example.test", extra: true }] },
    null,
    [],
  ]) assert.throws(() => validateLlmsOptions(options));
});

test("synthetic rendered HTML produces exact full and small Markdown", async () => {
  const html = [
    "<!-- retained? removed -->",
    "<h1>Heading</h1><p>First\tline <em>with emphasis</em>.</p>",
    "<table><thead><tr><th>Name</th><th>Value</th></tr></thead><tbody><tr><td>alpha</td><td>one</td></tr></tbody></table>",
    '<pre><code class="language-js">const value = 1;\n</code></pre>',
    '<aside class="starlight-aside starlight-aside--note"><p>NOTE_TEXT</p></aside>',
    '<aside class="starlight-aside starlight-aside--tip"><p>TIP_TEXT</p></aside>',
    '<aside class="starlight-aside starlight-aside--caution"><p>CAUTION_TEXT</p></aside>',
    '<aside class="starlight-aside starlight-aside--danger"><p>DANGER_TEXT</p></aside>',
    "<details><summary>More</summary><p>DETAILS_TEXT</p></details>",
    "<starlight-tabs><div role=\"tablist\"><button role=\"tab\"> JavaScript </button></div><div role=\"tabpanel\"><p>TAB_TEXT</p></div></starlight-tabs>",
  ].join("\n");
  const full = await entryToSimpleMarkdown(html, { profile: "full" });
  const small = await entryToSimpleMarkdown(html, { profile: "small" });
  assert.equal(full, "# Heading\n\nFirst line *with emphasis*.\n\n| Name  | Value |\n| ----- | ----- |\n| alpha | one   |\n\n```js\nconst value = 1;\n```\n\nNOTE_TEXT\n\nTIP_TEXT\n\nCAUTION_TEXT\n\nDANGER_TEXT\n\nMore\n\nDETAILS_TEXT\n\n* JavaScript\n\n  TAB_TEXT");
  assert.equal(small, "# Heading First line *with emphasis*. | Name | Value | | ----- | ----- | | alpha | one | \n```js\nconst value = 1;\n```\n CAUTION_TEXT DANGER_TEXT * JavaScript TAB_TEXT");
  assert.match(full, /NOTE_TEXT/);
  assert.match(full, /TIP_TEXT/);
  assert.match(full, /DETAILS_TEXT/);
  for (const text of ["NOTE_TEXT", "TIP_TEXT", "DETAILS_TEXT"]) assert.doesNotMatch(small, new RegExp(text));
  assert.match(small, /CAUTION_TEXT/);
  assert.match(small, /DANGER_TEXT/);
});

test("lock graph removes the vulnerable chain and retains the exact renderer graph", () => {
  const pkg = JSON.parse(readFileSync(path.join(root, "website/package.json"), "utf8"));
  const candidate = JSON.parse(readFileSync(path.join(root, "website/package-lock.json"), "utf8"));
  const forbidden = new Set(["starlight-llms-txt", "micromatch", "braces", "@types/micromatch", "@types/braces"]);
  const expectedDependencies = {
    "@astrojs/mdx": "8.0.2",
    "@astrojs/starlight": "^0.42.4",
    "astro": "^7.3.5",
    "hast-util-select": "6.0.4",
    "rehype-parse": "9.0.1",
    "rehype-remark": "10.0.1",
    "remark-gfm": "4.0.1",
    "remark-stringify": "11.0.0",
    "starlight-links-validator": "^0.26.0",
    "starlight-plugin-icons": "^1.1.6",
    "starlight-versions": "^0.10.1",
    "unified": "11.0.5",
    "unist-util-remove": "4.0.0",
  };
  const permittedRemovedPaths = [
    "node_modules/@types/braces",
    "node_modules/@types/micromatch",
    "node_modules/braces",
    "node_modules/fill-range",
    "node_modules/is-number",
    "node_modules/micromatch",
    "node_modules/micromatch/node_modules/picomatch",
    "node_modules/starlight-llms-txt",
    "node_modules/to-regex-range",
  ];
  const expectedLockSha256 = "18de86623839b1a581d46ced2895d0c7aca6fc056086a1c3a061da4407633d59";
  const expectedRetainedGraphSha256 = "4a2e21a4897afcb0f0b218f74b0233b25b5aae2c6f1f6329280660abe1bb4652";
  assert.deepEqual(pkg.dependencies, expectedDependencies, "package.json direct dependency map changed");
  assert.deepEqual(candidate.packages[""].dependencies, expectedDependencies, "lock root direct dependency map changed");
  assert.equal(createHash("sha256").update(readFileSync(path.join(root, "website/package-lock.json"))).digest("hex"), expectedLockSha256, "package lock content changed");
  for (const lockPath of permittedRemovedPaths) assert.equal(candidate.packages[lockPath], undefined, `permitted removed node reappeared: ${lockPath}`);
  const nameOf = (lockPath) => {
    const index = lockPath.lastIndexOf("node_modules/");
    if (index < 0) return "";
    const parts = lockPath.slice(index + "node_modules/".length).split("/");
    return parts[0].startsWith("@") ? `${parts[0]}/${parts[1]}` : parts[0];
  };
  const candidatePackages = candidate.packages ?? {};
  for (const [lockPath, entry] of Object.entries(candidatePackages)) {
    if (forbidden.has(nameOf(lockPath))) assert.fail(`forbidden lock node: ${lockPath}`);
    for (const field of ["dependencies", "optionalDependencies", "peerDependencies", "devDependencies"]) {
      for (const dependency of Object.keys(entry[field] ?? {})) {
        if (forbidden.has(dependency)) assert.fail(`forbidden ${field} edge ${lockPath} -> ${dependency}`);
      }
    }
  }
  const fingerprints = {
    "@astrojs/mdx": ["8.0.2", "https://registry.npmjs.org/@astrojs/mdx/-/mdx-8.0.2.tgz", "sha512-WedIVP2Bu8fXSzxb3ZYZ8xDrWWjf2SRKCneh52Llhoa+O3lcpHPhoZVP3FerlivUaH/0LJz2yVaCNvMdg0u8Bw=="],
    "hast-util-select": ["6.0.4", "https://registry.npmjs.org/hast-util-select/-/hast-util-select-6.0.4.tgz", "sha512-RqGS1ZgI0MwxLaKLDxjprynNzINEkRHY2i8ln4DDjgv9ZhcYVIHN9rlpiYsqtFwrgpYU361SyWDQcGNIBVu3lw=="],
    "rehype-parse": ["9.0.1", "https://registry.npmjs.org/rehype-parse/-/rehype-parse-9.0.1.tgz", "sha512-ksCzCD0Fgfh7trPDxr2rSylbwq9iYDkSn8TCDmEJ49ljEUBxDVCzCHv7QNzZOfODanX4+bWQ4WZqLCRWYLfhag=="],
    "rehype-remark": ["10.0.1", "https://registry.npmjs.org/rehype-remark/-/rehype-remark-10.0.1.tgz", "sha512-EmDndlb5NVwXGfUa4c9GPK+lXeItTilLhE6ADSaQuHr4JUlKw9MidzGzx4HpqZrNCt6vnHmEifXQiiA+CEnjYQ=="],
    "remark-gfm": ["4.0.1", "https://registry.npmjs.org/remark-gfm/-/remark-gfm-4.0.1.tgz", "sha512-1quofZ2RQ9EWdeN34S79+KExV1764+wCUGop5CPL1WGdD0ocPpu91lzPGbwWMECpEpd42kJGQwzRfyov9j4yNg=="],
    "remark-stringify": ["11.0.0", "https://registry.npmjs.org/remark-stringify/-/remark-stringify-11.0.0.tgz", "sha512-1OSmLd3awB/t8qdoEOMazZkNsfVTeY4fTsgzcQFdXNq8ToTN4ZGwrMnlda4K6smTFKD+GRV6O48i6Z4iKgPPpw=="],
    "unified": ["11.0.5", "https://registry.npmjs.org/unified/-/unified-11.0.5.tgz", "sha512-xKvGhPWw3k84Qjh8bI3ZeJjqnyadK+GEFtazSfZv/rKeTkTjOJho6mFqh2SM96iIcZokxiOpg78GazTSg8+KHA=="],
    "unist-util-remove": ["4.0.0", "https://registry.npmjs.org/unist-util-remove/-/unist-util-remove-4.0.0.tgz", "sha512-b4gokeGId57UVRX/eVKej5gXqGlc9+trkORhFJpu9raqZkZhU0zm8Doi05+HaiBsMEIJowL+2WtQ5ItjsngPXg=="],
  };
  for (const [name, [version, resolved, integrity]] of Object.entries(fingerprints)) {
    assert.equal(pkg.dependencies[name], version, `direct package.json version for ${name}`);
    const lock = candidatePackages[`node_modules/${name}`];
    assert.deepEqual([lock?.version, lock?.resolved, lock?.integrity], [version, resolved, integrity], name);
  }
  // The reviewed website cache update changes exactly these three fields.
  assert.deepEqual(candidatePackages["node_modules/http-cache-semantics"], {
    version: "4.3.0",
    resolved: "https://registry.npmjs.org/http-cache-semantics/-/http-cache-semantics-4.3.0.tgz",
    integrity: "sha512-M5t5LlJpS1UHMjvwRQVdFHvPISGeLAxNcrWuJkeGh0KxsqCHZ1O3NXZU/8x7cD0BDcGW8kapxMKTvwlqrNkHkA==",
    license: "BSD-2-Clause",
  });
  // Normalize the historical cache identity before hashing the freshly reviewed renderer graph.
  candidatePackages["node_modules/http-cache-semantics"] = {
    version: "4.2.0",
    resolved: "https://registry.npmjs.org/http-cache-semantics/-/http-cache-semantics-4.2.0.tgz",
    integrity: "sha512-dTxcvPXqPvXBQpq5dUr6mEMJX4oIEFv6bwom3FDwKRDsuIjjJGANqhBuoAn9c1RQJIdAKav33ED65E2ys+87QQ==",
    license: "BSD-2-Clause",
  };
  const candidateRoot = candidatePackages[""];
  const retainedGraph = Object.fromEntries(Object.entries(candidatePackages)
    .filter(([key]) => !permittedRemovedPaths.includes(key))
    .map(([key, entry]) => [key, key === "" ? Object.fromEntries(Object.entries(entry).filter(([field]) => field !== "dependencies")) : entry]));
  const sortRecursively = (value) => {
    if (Array.isArray(value)) return value.map(sortRecursively);
    if (value && typeof value === "object") {
      return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortRecursively(value[key])]));
    }
    return value;
  };
  const graphBytes = Buffer.from(JSON.stringify(sortRecursively(retainedGraph)), "utf8");
  assert.equal(createHash("sha256").update(graphBytes).digest("hex"), expectedRetainedGraphSha256, "retained lock graph changed");
});
