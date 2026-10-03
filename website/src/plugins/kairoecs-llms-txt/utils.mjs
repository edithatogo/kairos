export function getSiteTitle(context) {
  const defaultLang = (context.defaultLocale === "root" ? context.locales?.root?.lang : context.defaultLocale) || "en";
  return typeof context.title === "string" ? context.title : context.title[defaultLang];
}

export function ensureTrailingSlash(value) {
  return value.at(-1) === "/" ? value : `${value}/`;
}
