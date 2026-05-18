#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");
const websiteRoot = path.join(repoRoot, "website");
const justfilePath = path.join(repoRoot, "justfile");
const windowsBootstrapPath = path.join(repoRoot, "scripts", "bootstrap.ps1");
const packageJsonPath = path.join(websiteRoot, "package.json");
const buildIndexPath = path.join(websiteRoot, "build", "index.html");
const port = Number(process.env.DOCS_SMOKE_PORT || 41727);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readText(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      env: process.env,
      stdio: "inherit",
      ...options,
    });

    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(" ")} exited with ${code}`));
    });
  });
}

function runNpm(args) {
  if (process.platform === "win32") {
    return run("cmd.exe", ["/d", "/s", "/c", `npm ${args.join(" ")}`]);
  }
  return run("npm", args);
}

function waitForHttp(url, timeoutMs = 8000) {
  const deadline = Date.now() + timeoutMs;

  return new Promise((resolve, reject) => {
    const attempt = () => {
      const request = http.get(url, (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          body += chunk;
        });
        response.on("end", () => {
          if (response.statusCode === 200) {
            resolve(body);
            return;
          }
          retry();
        });
      });

      request.on("error", retry);
      request.setTimeout(1000, () => {
        request.destroy();
        retry();
      });
    };

    const retry = () => {
      if (Date.now() > deadline) {
        reject(new Error(`Timed out waiting for ${url}`));
        return;
      }
      setTimeout(attempt, 200);
    };

    attempt();
  });
}

async function smokeDevServer() {
  const server = http.createServer((request, response) => {
    if (request.url !== "/" && request.url !== "/index.html") {
      response.statusCode = 404;
      response.setHeader("Content-Type", "text/plain; charset=utf-8");
      response.end("Not found");
      return;
    }

    response.statusCode = 200;
    response.setHeader("Content-Type", "text/html; charset=utf-8");
    response.end(readText(buildIndexPath));
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", resolve);
  });

  try {
    const body = await waitForHttp(`http://127.0.0.1:${port}/`);
    assert(body.includes("KairoECS Documentation"), "docs preview did not render the docs title");
    assert(body.includes("just docs-build"), "docs preview did not include contributor docs commands");
    process.stdout.write(`Docs dev smoke passed at http://127.0.0.1:${port}/\n`);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

async function main() {
  const justfile = readText(justfilePath);
  for (const recipe of ["docs-bootstrap:", "docs-build:", "docs-dev:", "docs-smoke:", "check-docs:", "toolchain-docs:"]) {
    assert(justfile.includes(recipe), `missing ${recipe} recipe in justfile`);
  }
  assert(fs.existsSync(windowsBootstrapPath), "missing Windows bootstrap script");
  const windowsBootstrap = readText(windowsBootstrapPath);
  assert(windowsBootstrap.includes("CheckOnly"), "Windows bootstrap script is missing CheckOnly mode");
  assert(windowsBootstrap.includes("cargo install just --locked"), "Windows bootstrap script does not document just installation");

  const packageJson = JSON.parse(readText(packageJsonPath));
  assert(packageJson.scripts?.build === "node scripts/build.js", "website build script is not wired to scripts/build.js");
  assert(packageJson.scripts?.start === "node scripts/dev.js", "website start script is not wired to scripts/dev.js");

  const devScript = readText(path.join(websiteRoot, "scripts", "dev.js"));
  assert(devScript.includes("server.listen(port"), "website dev script does not listen on the configured port");

  const docsPlatform = readText(path.join(repoRoot, "docs", "developer-experience", "docs-platform.md"));
  assert(docsPlatform.includes("Astro/Starlight"), "docs-platform note does not mention the roadmap target");
  assert(docsPlatform.includes("custom Node"), "docs-platform note does not mention the current site");

  const coverageMatrix = readText(path.join(repoRoot, "docs", "tutorials", "coverage-matrix.md"));
  assert(coverageMatrix.includes("Learning Coverage Matrix"), "coverage matrix is missing its title");
  assert(coverageMatrix.includes("Python scheduler tutorial"), "coverage matrix is missing notebook coverage");
  assert(coverageMatrix.includes("The repository does not require a notebook for every language"), "coverage matrix does not explain notebook exclusions");

  const docsWorkflow = readText(path.join(repoRoot, "docs", "developer-experience", "docs-workflow.md"));
  assert(docsWorkflow.includes("validate-learning-coverage.mjs"), "docs workflow does not mention the coverage validator");

  const docsReadme = readText(path.join(repoRoot, "docs", "README.md"));
  assert(docsReadme.includes("docs/developer-experience/docs-platform.md"), "docs overview does not link the docs-platform note");
  assert(docsReadme.includes("docs/tutorials/coverage-matrix.md"), "docs overview does not link the learning-coverage matrix");

  const tutorialIndex = readText(path.join(repoRoot, "docs", "tutorials", "index.md"));
  assert(tutorialIndex.includes("coverage-matrix.md"), "tutorial index does not link the learning-coverage matrix");

  const notebookTutorials = readText(path.join(repoRoot, "docs", "tutorials", "notebooks.md"));
  assert(notebookTutorials.includes("python_scheduler_tutorial.ipynb"), "notebook tutorials page is missing the Python scheduler notebook");
  assert(notebookTutorials.includes("reproducible_benchmark_scenario.ipynb"), "notebook tutorials page is missing the benchmark notebook");
  assert(notebookTutorials.includes("colab_gpu_smoke.ipynb"), "notebook tutorials page is missing the Colab GPU notebook");
  assert(notebookTutorials.includes("colab_tpu_smoke.ipynb"), "notebook tutorials page is missing the Colab TPU notebook");
  assert(notebookTutorials.includes("colab_tpu_dedicated_smoke.ipynb"), "notebook tutorials page is missing the dedicated Colab TPU notebook");

  const websiteIndex = readText(path.join(websiteRoot, "src", "index.md"));
  assert(websiteIndex.includes("../../docs/developer-experience/docs-platform.md"), "website index does not link the docs-platform note");
  assert(websiteIndex.includes("../../docs/tutorials/coverage-matrix.md"), "website index does not link the learning-coverage matrix");

  await runNpm(["--prefix", "website", "run", "check:links"]);
  await runNpm(["--prefix", "website", "run", "build"]);

  assert(fs.existsSync(buildIndexPath), "missing website/build/index.html after docs build");
  const built = readText(buildIndexPath);
  assert(built.includes("KairoECS Documentation"), "built docs HTML is missing the docs title");
  assert(built.includes("just docs-dev"), "built docs HTML is missing the docs-dev command");

  await smokeDevServer();
  process.stdout.write("Docs workflow validation passed.\n");
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
