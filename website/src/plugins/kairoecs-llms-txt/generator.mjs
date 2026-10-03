/** Match the only configured upstream promote expression, micromatch `index*`. */
export function isPromotedIndexId(id) {
  return typeof id === "string" && id.startsWith("index") && !id.includes("/");
}

function localeLanguage(context) {
  return (context.defaultLocale === "root" ? context.locales?.root?.lang : context.defaultLocale) || "en";
}

function isDefaultLocale(id, context) {
  const lang = localeLanguage(context);
  const locales = Object.keys(context.locales || {}).filter((key) => key !== "root" && key !== lang);
  return !locales.some((key) => id === key || id.startsWith(`${key}/`));
}

function prioritizedId(id) {
  return isPromotedIndexId(id) ? `_${id}` : id;
}

let astroContainerPromise;
async function renderDocHtml(doc, astroContext) {
  const [{ render }, { experimental_AstroContainer }, { default: mdxServer }] = await Promise.all([
    import("astro:content"),
    import("astro/container"),
    import("@astrojs/mdx/server.js"),
  ]);
  astroContainerPromise ??= experimental_AstroContainer.create({
    renderers: [{ name: "astro:jsx", ssr: mdxServer }],
  });
  const { Content } = await render(doc);
  return (await astroContainerPromise).renderToString(Content, astroContext);
}

/** Generate the upstream-compatible Markdown document from the public docs collection. */
export async function generateLlmsTxt(astroContext, { minify, description }) {
  const [{ getCollection }, { kairoecsLlmsTxtContext: context }, { renderToMarkdown }] = await Promise.all([
    import("astro:content"),
    import("virtual:kairoecs-llms-txt/context"),
    import("./entry-to-simple-markdown.mjs"),
  ]);
  const defaultLang = localeLanguage(context);
  const collator = new Intl.Collator(defaultLang);
  const docs = await getCollection("docs", (doc) => isDefaultLocale(doc.id, context) && !doc.data.draft);
  docs.sort((a, b) => collator.compare(prioritizedId(a.id), prioritizedId(b.id)));
  const segments = [];
  for (const doc of docs) {
    const docSegments = [`# ${doc.data.hero?.title || doc.data.title}`];
    const summary = doc.data.hero?.tagline || doc.data.description;
    if (summary) docSegments.push(`> ${summary}`);
    const html = await renderDocHtml(doc, astroContext);
    docSegments.push(await renderToMarkdown(html, minify ? "small" : "full"));
    segments.push(docSegments.join("\n\n"));
  }
  if (description) segments.unshift(`<SYSTEM>${description}</SYSTEM>`);
  return segments.join(context.pageSeparator);
}

export { isDefaultLocale };
