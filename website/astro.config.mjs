import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightVersions from "starlight-versions";
import starlightLinksValidator from "starlight-links-validator";
import starlightLlmsTxt from "starlight-llms-txt";
import { starlightIconsPlugin } from "starlight-plugin-icons";
import polyglotPlugin from "./src/plugins/starlight-polyglot.mjs";

const repo = "https://github.com/edithatogo/kairos";

export default defineConfig({
  site: "https://edithatogo.github.io",
  base: "/kairos",
  outDir: "./build",
  integrations: [
    starlight({
      title: "KairoECS",
      description:
        "Deterministic simulation engine docs for Rust, Python, R, Julia, TypeScript/Wasm, C#, and Go.",
      editLink: {
        baseUrl: `${repo}/edit/main/website/src/content/docs/`,
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: repo,
        },
      ],
      sidebar: [
        {
          label: "Start",
          items: [
            { label: "Overview", slug: "index" },
            { label: "Architecture", slug: "architecture" },
            { label: "Docs Platform", slug: "docs-platform" },
          ],
        },
        {
          label: "Polyglot",
          items: [
            { label: "Rust", slug: "polyglot/rust" },
            { label: "Python", slug: "polyglot/python" },
            { label: "R", slug: "polyglot/r" },
            { label: "Julia", slug: "polyglot/julia" },
            { label: "TypeScript/Wasm", slug: "polyglot/typescript-wasm" },
            { label: "C#", slug: "polyglot/csharp" },
            { label: "Go", slug: "polyglot/go" },
          ],
        },
        {
          label: "Evidence",
          items: [
            { label: "Conductor Status", slug: "evidence/conductor-status" },
            { label: "GPU and WebGPU", slug: "evidence/gpu-webgpu" },
            { label: "PDES and Distributed", slug: "evidence/pdes-distributed" },
            { label: "Cloud/HPC Runtime Boundary", slug: "evidence/cloud-hpc" },
          ],
        },
      ],
      plugins: [
        polyglotPlugin({
          languages: ["Rust", "Python", "R", "Julia", "TypeScript/Wasm", "C#", "Go"],
          sourceRoot: repo,
        }),
        starlightVersions({
          current: { label: "R2 Preview" },
          versions: [{ slug: "r1", label: "R1 Archive", redirect: "root" }],
        }),
        starlightLinksValidator({
          errorOnRelativeLinks: false,
          failOnError: false,
          reporters: { githubActions: true, json: true },
        }),
        starlightLlmsTxt({
          projectName: "KairoECS",
          details:
            "KairoECS documentation covers deterministic simulation, conformance evidence, and polyglot bindings.",
          optionalLinks: [{ label: "Repository", url: repo }],
        }),
        starlightIconsPlugin({ sidebar: true, codeblock: true }),
      ],
    }),
  ],
});
