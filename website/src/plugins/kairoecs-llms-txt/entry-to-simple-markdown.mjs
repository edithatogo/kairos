import { matches, select, selectAll } from "hast-util-select";
import rehypeParse from "rehype-parse";
import rehypeRemark from "rehype-remark";
import remarkGfm from "remark-gfm";
import remarkStringify from "remark-stringify";
import { unified } from "unified";
import { remove } from "unist-util-remove";

function improveExpressiveCode(tree) {
  const instances = selectAll(".expressive-code", tree);
  for (const instance of instances) {
    const figcaption = select("figcaption", instance);
    if (figcaption) {
      const index = figcaption.children.findIndex((child) => matches("span.sr-only", child));
      if (index > -1) figcaption.children.splice(index, 1);
    }
    const pre = select("pre", instance);
    const code = select("code", instance);
    if (pre?.properties.dataLanguage && code) {
      if (!Array.isArray(code.properties.className)) code.properties.className = [];
      const diffLines = pre.properties.dataLanguage === "diff"
        ? []
        : code.children.filter((child) => matches("div.ec-line.ins, div.ec-line.del", child));
      if (diffLines.length === 0) {
        code.properties.className.push(`language-${pre.properties.dataLanguage}`);
      } else {
        code.properties.className.push("language-diff");
        for (const line of diffLines) {
          if (line.type !== "element") continue;
          const classes = line.properties?.className;
          if (typeof classes !== "string" && !Array.isArray(classes)) continue;
          const marker = classes.includes("ins") ? "+" : "-";
          const span = select("span:not(.indent)", line);
          const firstChild = span?.children[0];
          if (firstChild?.type === "text") firstChild.value = `${marker}${firstChild.value}`;
        }
      }
    }
  }
}

function improveTabs(tree) {
  for (const instance of selectAll("starlight-tabs", tree)) {
    const tabs = selectAll('[role="tab"]', instance);
    const panels = selectAll('[role="tabpanel"]', instance);
    instance.tagName = "ul";
    instance.properties = {};
    instance.children = [];
    for (let index = 0; index < Math.min(tabs.length, panels.length); index++) {
      const tab = tabs[index];
      const panel = panels[index];
      if (!tab || !panel) continue;
      const tabLabel = tab.children
        .filter((child) => child.type === "text" && child.value.trim())
        .map((child) => child.value.trim())
        .join("");
      instance.children.push({
        type: "element",
        tagName: "li",
        properties: {},
        children: [
          { type: "element", tagName: "p", properties: {}, children: [{ type: "text", value: tabLabel }] },
          panel,
        ],
      });
    }
  }
}

function improveFileTrees(tree) {
  for (const fileTree of selectAll("starlight-file-tree", tree)) {
    remove(fileTree, (node) => matches(".sr-only", node));
  }
}

const pipeline = unified()
  .use(rehypeParse, { fragment: true })
  .use(function localLlmsTransforms() {
    return (tree, file) => {
      if (file.data.kairoecsLlmsTxt.profile === "small") {
        remove(tree, (node) => matches("details", node) ||
          (matches(".starlight-aside", node) &&
            (matches(".starlight-aside--note", node) || matches(".starlight-aside--tip", node))));
      }
      improveExpressiveCode(tree);
      improveTabs(tree);
      improveFileTrees(tree);
      remove(tree, (node) => node.type === "comment");
      return tree;
    };
  })
  .use(rehypeRemark)
  .use(remarkGfm)
  .use(remarkStringify);

function collapseProseWhitespace(markdown) {
  const matcher = /(?<=^|\n)([ \t]*)(`{3,}|~{3,})[^\n]*\n(?:[\s\S]*?\n)?\1\2[ \t]*(?=\n|$)/g;
  const parts = [];
  let lastIndex = 0;
  for (const match of markdown.matchAll(matcher)) {
    const index = match.index ?? 0;
    parts.push(markdown.slice(lastIndex, index).replace(/\s+/g, " "));
    parts.push("\n", match[0], "\n");
    lastIndex = index + match[0].length;
  }
  parts.push(markdown.slice(lastIndex).replace(/\s+/g, " "));
  return parts.join("").trim();
}

export async function renderToMarkdown(html, profile = "full") {
  if (profile !== "full" && profile !== "small") throw new TypeError(`unknown llms output profile: ${profile}`);
  let markdown = String(await pipeline.process({ value: html, data: { kairoecsLlmsTxt: { profile } } })).trim();
  if (profile === "small") markdown = collapseProseWhitespace(markdown);
  return markdown;
}

export async function entryToSimpleMarkdown(html, { profile = "full" } = {}) {
  return renderToMarkdown(html, profile);
}
