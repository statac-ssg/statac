const markdownIt = require("markdown-it");
const pluginRss = require("@11ty/eleventy-plugin-rss");

module.exports = function (eleventyConfig) {
  // Copied as they are.
  eleventyConfig.addPassthroughCopy("src/img");
  eleventyConfig.addPassthroughCopy({ "fonts": "assets/fonts" });
  eleventyConfig.addPlugin(pluginRss);
  eleventyConfig.addFilter("readableDate", (date) => date);
  eleventyConfig.addCollection("featured", (api) => api.getAll());
  eleventyConfig.setLibrary(
    "md",
    markdownIt({ html: true, breaks: true, linkify: true }).use(require("markdown-it-attrs"))
  );
  eleventyConfig.setFrontMatterParsingOptions({ excerpt: true, excerpt_separator: "<!-- excerpt -->" });
  return {
    dir: { input: "src", output: "dist", includes: "parts", data: "info" },
    markdownTemplateEngine: "njk",
    pathPrefix: "/",
  };
};
