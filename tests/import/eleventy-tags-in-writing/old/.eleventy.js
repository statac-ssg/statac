module.exports = function (eleventyConfig) {
  eleventyConfig.addShortcode("image", (src, alt) => `<img src="${src}" alt="${alt}">`);
  eleventyConfig.addPairedShortcode("aside", (content, kind) => `<aside>${content}</aside>`);
};
