import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { stripTypeScriptTypes } from "node:module";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const srcPath = resolve(packageRoot, "src", "index.ts");
const declarationPath = resolve(packageRoot, "src", "index.d.ts");
const distPath = resolve(packageRoot, "dist");

const source = await readFile(srcPath, "utf8");
const declarations = await readFile(declarationPath, "utf8");

await mkdir(distPath, { recursive: true });
await writeFile(resolve(distPath, "index.js"), stripTypeScriptTypes(source, { mode: "strip" }));
await writeFile(resolve(distPath, "index.d.ts"), declarations);
