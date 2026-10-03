import { generateLlmsTxt } from "../generator.mjs";
import { getSiteTitle } from "../utils.mjs";

export const prerender = true;
export const GET = async (context) => new Response(await generateLlmsTxt(context, {
  minify: false,
  description: `This is the full developer documentation for ${getSiteTitle((await import("virtual:kairoecs-llms-txt/context")).kairoecsLlmsTxtContext)}`,
}));
