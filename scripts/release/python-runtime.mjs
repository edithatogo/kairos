import path from "node:path";
import { spawnSync } from "node:child_process";

export function releasePython(repoRoot) {
  const executable = process.platform === "win32" ? ["Scripts", "python.exe"] : ["bin", "python"];
  const python = path.join(repoRoot, ".venv", ...executable);
  const check = spawnSync(python, ["-I", "scripts/python_runtime.py"], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  if (check.status !== 0) {
    throw new Error("Release checks require the verified checkout-local Python 3.15.2 environment.");
  }
  return python;
}
