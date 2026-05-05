export const PACKAGE_NAME = "@kairo-ecs/typescript" as const;
export const BINDING_KIND = "typescript-wasm" as const;

export interface BindingSurfaceInfo {
  readonly packageName: typeof PACKAGE_NAME;
  readonly bindingKind: typeof BINDING_KIND;
  readonly version: string;
  readonly runtimeTargets: readonly ("node" | "browser")[];
}

export interface BindingSurfaceOptions {
  readonly version?: string;
  readonly runtimeTargets?: readonly ("node" | "browser")[];
}

export function createBindingSurfaceInfo(options: BindingSurfaceOptions = {}): BindingSurfaceInfo {
  const version = normalizeVersion(options.version ?? "0.1.0");
  const runtimeTargets = normalizeRuntimeTargets(options.runtimeTargets ?? ["node", "browser"]);

  return {
    packageName: PACKAGE_NAME,
    bindingKind: BINDING_KIND,
    version,
    runtimeTargets,
  };
}

export function normalizeVersion(value: string): string {
  const trimmed = value.trim();

  if (trimmed.length === 0) {
    throw new Error("version must not be empty");
  }

  return trimmed;
}

export function normalizeRuntimeTargets(
  value: readonly ("node" | "browser")[],
): readonly ("node" | "browser")[] {
  const seen = new Set<"node" | "browser">();
  const normalized: ("node" | "browser")[] = [];

  for (const target of value) {
    if (target !== "node" && target !== "browser") {
      throw new Error(`unsupported runtime target: ${String(target)}`);
    }

    if (!seen.has(target)) {
      seen.add(target);
      normalized.push(target);
    }
  }

  return normalized;
}

export function describeBindingSurface(options: BindingSurfaceOptions = {}): string {
  const info = createBindingSurfaceInfo(options);
  return `${info.packageName} [${info.bindingKind}] ${info.version} => ${info.runtimeTargets.join(", ")}`;
}
