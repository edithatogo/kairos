const VIRTUAL_CONTEXT = "virtual:kairoecs-llms-txt/context";
const UNKNOWN = Symbol("unknown option");
const SUPPORTED = new Set(["projectName", "description", "details", "optionalLinks"]);

export function validateLlmsOptions(options) {
  if (options === null || typeof options !== "object" || Array.isArray(options)) {
    throw new TypeError("kairoecs-llms-txt options must be an object");
  }
  for (const key of Object.keys(options)) {
    if (!SUPPORTED.has(key)) {
      const hint = ["customSets", "include", "exclude", "promote", "demote", "minify", "customSelectors", "pageSeparator", "rawContent"].includes(key)
        ? " is unsupported by the local integration"
        : " is not a supported option";
      throw new TypeError(`kairoecs-llms-txt option ${key}${hint}`);
    }
  }
  for (const key of ["projectName", "description", "details"]) {
    if (options[key] !== undefined && typeof options[key] !== "string") {
      throw new TypeError(`kairoecs-llms-txt option ${key} must be a string`);
    }
  }
  if (options.optionalLinks !== undefined) {
    if (!Array.isArray(options.optionalLinks)) {
      throw new TypeError("kairoecs-llms-txt option optionalLinks must be an array");
    }
    for (const [index, link] of options.optionalLinks.entries()) {
      if (!link || typeof link !== "object" || Array.isArray(link) ||
          typeof link.label !== "string" || typeof link.url !== "string" ||
          (link.description !== undefined && typeof link.description !== "string") ||
          Object.keys(link).some((key) => !["label", "url", "description"].includes(key))) {
        throw new TypeError(`kairoecs-llms-txt optionalLinks[${index}] must have label/url strings and an optional description`);
      }
    }
  }
  return { ...options, optionalLinks: options.optionalLinks ?? [] };
}

function virtualId(id) {
  return `\0${id}`;
}

export default function kairoecsLlmsTxt(input = {}) {
  const options = validateLlmsOptions(input);
  return {
    name: "kairoecs-llms-txt",
    hooks: {
      setup({ astroConfig, addIntegration, config }) {
        if (!astroConfig.site) {
          throw new Error("kairoecs-llms-txt requires `site` in Astro configuration");
        }
        addIntegration({
          name: "kairoecs-llms-txt",
          hooks: {
            "astro:config:setup"({ injectRoute, updateConfig }) {
              for (const [entrypoint, pattern] of [
                ["./routes/llms.txt.mjs", "/llms.txt"],
                ["./routes/llms-full.txt.mjs", "/llms-full.txt"],
                ["./routes/llms-small.txt.mjs", "/llms-small.txt"],
              ]) {
                injectRoute({ entrypoint: new URL(entrypoint, import.meta.url), pattern, prerender: true });
              }
              const context = {
                base: astroConfig.base,
                title: options.projectName ?? config.title,
                description: options.description ?? config.description,
                details: options.details,
                optionalLinks: options.optionalLinks,
                defaultLocale: config.defaultLocale,
                locales: config.locales,
                pageSeparator: "\n\n",
              };
              const modules = { [VIRTUAL_CONTEXT]: `export const kairoecsLlmsTxtContext = ${JSON.stringify(context)};` };
              const resolutions = Object.fromEntries(Object.keys(modules).map((id) => [virtualId(id), id]));
              updateConfig({
                vite: {
                  plugins: [{
                    name: "vite-plugin-kairoecs-llms-txt-context",
                    resolveId(id) { if (id in modules) return virtualId(id); },
                    load(id) { const original = resolutions[id]; if (original) return modules[original]; },
                  }],
                },
              });
            },
          },
        });
      },
    },
  };
}
