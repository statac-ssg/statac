const markdownIt = require("markdown-it");
module.exports = function (eleventyConfig) {
  eleventyConfig.amendLibrary("md", (md) => md.set({ breaks: true }));
};
