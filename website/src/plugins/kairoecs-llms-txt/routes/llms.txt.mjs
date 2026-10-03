import { kairoecsLlmsTxtContext } from "virtual:kairoecs-llms-txt/context";
import { ensureTrailingSlash, getSiteTitle } from "../utils.mjs";

export const prerender = true;
export const GET = async (context) => {
  const title = getSiteTitle(kairoecsLlmsTxtContext);
  const description = kairoecsLlmsTxtContext.description ? `> ${kairoecsLlmsTxtContext.description}` : "";
  const site = new URL(ensureTrailingSlash(kairoecsLlmsTxtContext.base), context.site);
  const full = new URL("./llms-full.txt", site);
  const small = new URL("./llms-small.txt", site);
  const segments = [`# ${title}`];
  if (description) segments.push(description);
  if (kairoecsLlmsTxtContext.details) segments.push(kairoecsLlmsTxtContext.details);
  segments.push("## Documentation Sets");
  segments.push([
    `- [Abridged documentation](${small}): a compact version of the documentation for ${title}, with non-essential content removed`,
    `- [Complete documentation](${full}): the full documentation for ${title}`,
  ].join("\n"));
  segments.push("## Notes");
  segments.push("- The complete documentation includes all content from the official documentation\n- The content is automatically generated from the same source as the official documentation");
  if (kairoecsLlmsTxtContext.optionalLinks.length > 0) {
    segments.push("## Optional");
    segments.push(kairoecsLlmsTxtContext.optionalLinks.map((link) =>
      `- [${link.label}](${link.url})${link.description ? `: ${link.description}` : ""}`,
    ).join("\n"));
  }
  return new Response(`${segments.join("\n\n")}\n`);
};
