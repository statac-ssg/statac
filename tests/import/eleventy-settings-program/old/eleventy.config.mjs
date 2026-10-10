import settings from "./settings/index.js";

const folders = settings.folders();

export default async function (eleventyConfig) {
  await settings.apply(eleventyConfig);
  return { dir: folders, markdownTemplateEngine: settings.engine };
}
