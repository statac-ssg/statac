// Runs the search script a build wrote, reading the files beside it from
// the disk, and prints what each search asked finds: for each, the pages
// found in all, whether they hold only some of the words, and each result
// shown as its address, title, day, heading and excerpt.
//
//   node search_reader.mjs <site>/search/index.js '[["words", typing], ...]' [later.js]
//
// Given `later.js`, the script is read, then `later.js` written over it
// before anything is searched, as a browser holding the script from an
// earlier build of the site meets the files of a later one.

import { copyFile, readFile } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";

globalThis.fetch = async (url) => {
  try {
    return new Response(await readFile(fileURLToPath(url)));
  } catch {
    return new Response(null, { status: 404 });
  }
};

const [script, asked, later] = process.argv.slice(2);
const { search } = await import(pathToFileURL(script).href);
if (later) {
  await copyFile(later, script);
}
const found = [];
for (const [words, typing] of JSON.parse(asked)) {
  const { count, partial, results } = await search(words, { shown: 100, typing });
  found.push([
    count,
    partial,
    results.map(({ url, title, day, heading, excerpt }) => [
      url,
      title,
      day,
      heading,
      excerpt.map(({ text, marked }) => [text, marked]),
    ]),
  ]);
}
process.stdout.write(JSON.stringify(found));
