import markdownIt from 'markdown-it';

export default function (eleventyConfig) {
  eleventyConfig.addPassthroughCopy('files');
  eleventyConfig.addShortcode('year', () => '2026');
  eleventyConfig.setLibrary('md', markdownIt({ html: false, breaks: false, linkify: false }));
  eleventyConfig.setFrontMatterParsingOptions({ excerpt: true, excerpt_separator: '<!-- cut -->' });
}

export const config = {
  dir: { input: 'site', output: 'built', includes: 'layouts', data: 'data' },
  markdownTemplateEngine: 'liquid',
};
