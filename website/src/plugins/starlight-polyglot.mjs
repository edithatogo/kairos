export default function polyglotPlugin(options = {}) {
  const languages = options.languages ?? [];
  const sourceRoot = options.sourceRoot ?? "";

  return {
    name: "kairoecs-starlight-polyglot",
    hooks: {
      "config:setup"({ config, updateConfig }) {
        updateConfig({
          head: [
            ...(config.head ?? []),
            {
              tag: "meta",
              attrs: {
                name: "kairoecs-polyglot-languages",
                content: languages.join(", "),
              },
            },
            {
              tag: "meta",
              attrs: {
                name: "kairoecs-polyglot-source",
                content: sourceRoot,
              },
            },
          ],
        });
      },
    },
  };
}
