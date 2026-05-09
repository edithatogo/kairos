import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const distPath = resolve(packageRoot, "dist", "index.js");

const html = `<!doctype html>
<meta charset="utf-8">
<title>Kairo ECS browser smoke</title>
<script type="module">
  import {
    createBindingSurfaceInfo,
    createSchedulerFacade,
    loadNativeWasm,
    NativeWasmNotConfiguredError,
  } from "/dist/index.js";

  const scheduler = createSchedulerFacade();
  scheduler.scheduleAt({ timeTicks: 2n, priority: 0, eventKind: "browser" });
  const [event] = scheduler.runFor(1);

  let wasmErrorName = "";
  try {
    await loadNativeWasm();
  } catch (error) {
    wasmErrorName = error instanceof NativeWasmNotConfiguredError ? error.name : String(error);
  }

  globalThis.__kairoSmoke = {
    packageName: createBindingSurfaceInfo({ runtimeTargets: ["browser"] }).packageName,
    runtimeTargets: createBindingSurfaceInfo({ runtimeTargets: ["browser"] }).runtimeTargets,
    eventKind: event.eventKind,
    now: scheduler.currentTimeTicks.toString(),
    wasmErrorName,
  };
</script>`;

const server = createServer(async (request, response) => {
  try {
    if (request.url === "/" || request.url === "/browser-smoke.html") {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(html);
      return;
    }

    if (request.url === "/dist/index.js") {
      response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
      response.end(await readFile(distPath, "utf8"));
      return;
    }

    response.writeHead(404);
    response.end("not found");
  } catch (error) {
    console.error("browser smoke server failed", error);
    response.writeHead(500);
    response.end("internal server error");
  }
});

await new Promise((resolveServer) => {
  server.listen(0, "127.0.0.1", resolveServer);
});

const address = server.address();
assert(address !== null && typeof address === "object");

let browser;
try {
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  page.on("pageerror", (error) => {
    throw error;
  });
  await page.goto(`http://127.0.0.1:${address.port}/browser-smoke.html`);
  await page.waitForFunction(() => globalThis.__kairoSmoke !== undefined);
  const result = await page.evaluate(() => globalThis.__kairoSmoke);

  assert.equal(result.packageName, "@kairo-ecs/typescript");
  assert.deepEqual(result.runtimeTargets, ["browser"]);
  assert.equal(result.eventKind, "browser");
  assert.equal(result.now, "2");
  assert.equal(result.wasmErrorName, "NativeWasmNotConfiguredError");
} finally {
  await browser?.close();
  await new Promise((resolveClose, rejectClose) => {
    server.close((error) => {
      if (error) {
        rejectClose(error);
      } else {
        resolveClose(undefined);
      }
    });
  });
}
