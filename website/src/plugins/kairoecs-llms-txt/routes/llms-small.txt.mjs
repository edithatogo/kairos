export const prerender = true;
export const GET = async (context) => {
  const [{ generateLlmsTxt }, { getSiteTitle }, { kairoecsLlmsTxtContext }] = await Promise.all([
    import("../generator.mjs"),
    import("../utils.mjs"),
    import("virtual:kairoecs-llms-txt/context"),
  ]);
  const body = await generateLlmsTxt(context, {
    minify: true,
    description: `This is the abridged developer documentation for ${getSiteTitle(kairoecsLlmsTxtContext)}`,
  });
  return new Response(body);
};
