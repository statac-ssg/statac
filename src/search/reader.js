// Reads the search index of one of the site's languages and ranks what it
// finds, as `statac search` does. Statac writes it at each build, after
// the names of the index's files. A site's own script imports it and
// calls `search`.

const K1 = 1.2;
const B = 0.75;
// How much a word being typed counts in a longer word it starts.
const STARTING = 0.8;
// About how many UTF-16 code units of text are shown around a match.
const BEFORE = 60;
const AFTER = 140;

const MARKS = /[̀-ͯ᪰-᫿᷀-᷿⃐-⃿︠-︯]/gu;
// Chinese, Japanese and Korean, where one letter may start many words.
const UNSPACED =
  /[ᄀ-ᇿ぀-ヿ㄰-㆏ㇰ-ㇿ㐀-䶿一-鿿가-힯豈-﫿ｦ-ﾟ\u{20000}-\u{2ffff}]/u;
const SEGMENTER = globalThis.Intl?.Segmenter
  ? new Intl.Segmenter(undefined, { granularity: "word" })
  : null;

class Stale extends Error {}

/** Where each word of `text` is, as [start, end] in UTF-16 code units. */
function spans(text) {
  if (SEGMENTER) {
    const words = [...SEGMENTER.segment(text)].filter((piece) => piece.isWordLike);
    return words.map((piece) => [piece.index, piece.index + piece.segment.length]);
  }
  const words = [...text.matchAll(/[\p{L}\p{N}\p{M}]+(?:['’][\p{L}\p{N}\p{M}]+)*/gu)];
  return words.map((found) => [found.index, found.index + found[0].length]);
}

/** `word` in small letters, without the marks Latin, Greek and Cyrillic carry. */
function fold(word) {
  return word.normalize("NFKD").replace(MARKS, "").normalize("NFC").toLowerCase();
}

const read = new Map();

/** The JSON of the index's file `name`, unpacked. */
function unpacked(name) {
  if (!read.has(name)) {
    read.set(
      name,
      (async () => {
        const response = await fetch(new URL(name, import.meta.url));
        if (!response.ok) {
          throw new Stale(name);
        }
        let bytes = new Uint8Array(await response.arrayBuffer());
        // Packed with zlib, unless the host unpacked it on the way.
        if (bytes[0] === 0x78) {
          const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate"));
          bytes = new Uint8Array(await new Response(stream).arrayBuffer());
        }
        return JSON.parse(new TextDecoder().decode(bytes));
      })(),
    );
  }
  return read.get(name);
}

/** The place of the piece of the word list `word` falls in. */
function pieceOf(word) {
  let at = 0;
  while (at + 1 < FILES.words.length && FILES.words[at + 1][0] <= word) {
    at += 1;
  }
  return at;
}

/** The first place in the sorted `words` not before `word`. */
function from(words, word) {
  let low = 0;
  let high = words.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if (words[middle] < word) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low;
}

/**
 * The records lists `word` looks in, each with how much a match there
 * counts, adding to `marked` every word sharing a stem with it. A word
 * being typed looks in those of the words it starts too.
 */
async function lookedIn(word, typing, marked) {
  const looked = [];
  for (let at = pieceOf(word); at < FILES.words.length; at += 1) {
    if (at > pieceOf(word) && !(typing && FILES.words[at][0].startsWith(word))) {
      break;
    }
    const piece = await unpacked(FILES.words[at][1]);
    const counts = new Map();
    for (let place = from(piece.words, word); place < piece.words.length; place += 1) {
      const listed = piece.words[place];
      if (listed !== word && !(typing && listed.startsWith(word))) {
        break;
      }
      const stem = piece.stems[place];
      counts.set(stem, Math.max(counts.get(stem) ?? 0, listed === word ? 1 : STARTING));
    }
    for (const [stem, count] of counts) {
      looked.push([piece.records[stem], count]);
    }
    piece.words.forEach((listed, place) => {
      if (counts.has(piece.stems[place])) {
        marked.add(listed);
      }
    });
  }
  return looked;
}

/** The text around the first word in `text` that is `marked`, or its start, cut at words. */
function excerpt(text, marked) {
  const found = spans(text);
  const isMarked = ([start, end]) => marked.has(fold(text.slice(start, end)));
  const first = Math.max(found.findIndex(isMarked), 0);
  if (!found.length) {
    return [];
  }
  const at = found[first];
  let start = at[0];
  for (let place = first - 1; place >= 0 && at[0] - found[place][0] <= BEFORE; place -= 1) {
    start = found[place][0];
  }
  let last = at[1];
  for (let place = first; place < found.length && found[place][1] - start <= BEFORE + AFTER; place += 1) {
    last = found[place][1];
  }
  // On to the space after the last word, so its stop goes with it.
  const space = text.slice(last).search(/\s/u);
  const end = space < 0 ? text.length : last + space;
  const pieces = [];
  if (start > 0) {
    pieces.push({ text: "…", marked: false });
  }
  let written = start;
  for (const span of found) {
    if (start <= span[0] && span[1] <= end && isMarked(span)) {
      if (written < span[0]) {
        pieces.push({ text: text.slice(written, span[0]), marked: false });
      }
      pieces.push({ text: text.slice(span[0], span[1]), marked: true });
      written = span[1];
    }
  }
  if (written < end) {
    pieces.push({ text: text.slice(written, end), marked: false });
  }
  if (end < text.length) {
    pieces.push({ text: "…", marked: false });
  }
  return pieces;
}

/**
 * The pages found for `words`, best first, each once at its best part.
 * `shown` is how many to give in full; `typing` looks for the last word
 * as the start of longer words too, while it is being typed. Gives
 * `count`, the pages found in all; `partial`, true where no page holds
 * every word and those found hold some; and `results`, each with its
 * `url`; `title`, what a list calls the page; `heading`; `date`, a day as
 * `2026-10-19`; `day`, the date as the site writes it, none where the page
 * is called by it; and `excerpt`, a list of pieces of text each saying
 * whether it is `marked` as a word asked for.
 */
export async function search(words, { shown = 10, typing = false } = {}) {
  try {
    return await found(words, shown, typing);
  } catch (error) {
    // Read from an earlier build, whose files the site no longer holds.
    const fresh = new URL(import.meta.url);
    if (!(error instanceof Stale) || fresh.searchParams.has("fresh")) {
      throw error;
    }
    fresh.searchParams.set("fresh", Date.now());
    return (await import(fresh.href)).search(words, { shown, typing });
  }
}

async function found(query, shown, typing) {
  const located = spans(query);
  const all = located.map(([start, end]) => fold(query.slice(start, end)));
  const lastly = located[located.length - 1];
  const typed =
    typing && lastly && lastly[1] === query.length ? all[all.length - 1] : null;
  const being =
    typed && ([...typed].length > 1 || UNSPACED.test(typed)) ? typed : null;
  const asked = [...new Set(all)].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  if (!asked.length) {
    return { count: 0, partial: false, results: [] };
  }
  const marked = new Set();
  const [records, ...looked] = await Promise.all([
    unpacked(FILES.records),
    ...asked.map((word) => lookedIn(word, word === being, marked)),
  ]);
  const lengths = records.lengths;
  const count = lengths.length;
  let total = 0;
  for (const length of lengths) {
    total += length;
  }
  const average = count ? Math.max(total / count, 1) : 1;
  const held = new Map();
  for (const lists of looked) {
    const best = new Map();
    for (const [list, counts] of lists) {
      const holding = list.length / 2;
      const rarity = Math.log(1 + (count - holding + 0.5) / (holding + 0.5));
      let record = 0;
      for (let at = 0; at < list.length; at += 2) {
        record += list[at];
        const weight = list[at + 1];
        const length = lengths[record] / average;
        const score = (rarity * weight * (K1 + 1)) / (weight + K1 * (1 - B + B * length));
        best.set(record, Math.max(best.get(record) ?? 0, score * counts));
      }
    }
    for (const [record, score] of best) {
      const kept = held.get(record) ?? [0, 0];
      held.set(record, [kept[0] + 1, kept[1] + score]);
    }
  }
  const ranked = [...held].sort(
    ([a, [aMatched, aScore]], [b, [bMatched, bScore]]) =>
      bMatched - aMatched || bScore - aScore || a - b,
  );
  const most = ranked.length ? ranked[0][1][0] : 0;
  // Each record's page, by its place among the pages, and its part there.
  const pages = [];
  records.pages.forEach(([, parts], page) => {
    for (let part = 0; part < parts; part += 1) {
      pages.push([page, part]);
    }
  });
  const seen = new Set();
  const best = [];
  for (const [record, [matched]] of ranked) {
    if (matched < most) {
      break;
    }
    if (!seen.has(pages[record][0])) {
      seen.add(pages[record][0]);
      best.push(record);
    }
  }
  const results = await Promise.all(
    best.slice(0, shown).map(async (record) => {
      const [page, part] = pages[record];
      const text = await unpacked(`pages/${records.pages[page][0]}`);
      const [id, heading, words] = text.parts[part];
      return {
        url: id === null ? text.url : `${text.url}#${id}`,
        title: text.title,
        day: text.day,
        heading,
        date: text.date,
        excerpt: excerpt(words, marked),
      };
    }),
  );
  return { count: best.length, partial: most < asked.length, results };
}
