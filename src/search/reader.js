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
// The fewest characters a word respelt has, and the fewest it has to be
// two edits from another; a shorter word may be one.
const SHORTEST = 3;
const TWO_EDITS = 5;
// The most code points a word a run of Chinese, Japanese or Korean is read
// as has.
const LONGEST = 16;

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

/** Whether the index lists `word`, folded. */
async function listed(word) {
  const piece = await unpacked(FILES.words[pieceOf(word)][1]);
  return piece.words[from(piece.words, word)] === word;
}

/** Whether `word`, folded, starts a word the index lists. */
async function starts(word) {
  const at = pieceOf(word);
  const piece = await unpacked(FILES.words[at][1]);
  const place = from(piece.words, word);
  // Past the piece's last word, the next listed is the next piece's first.
  const next = place < piece.words.length ? piece.words[place] : FILES.words[at + 1]?.[0];
  return next?.startsWith(word) ?? false;
}

/**
 * The run of words at `located` in `text`, each next to the one before,
 * read as the fewest words the index lists, folded, cut where the words
 * are cut or anywhere in a word it doesn't list; between readings of as
 * many, the one whose first word is longest, then whose second is, and so
 * on. Where `typing`, its last word may be one that only starts a word
 * listed. Null where no reading covers the run.
 */
async function readRun(text, located, typing) {
  const whole = await Promise.all(
    located.map(([start, end]) => listed(fold(text.slice(start, end)))),
  );
  const cuts = [];
  located.forEach(([start, end], at) => {
    cuts.push(start);
    if (!whole[at]) {
      let place = start;
      for (const letter of text.slice(start, end)) {
        if (place > start) {
          cuts.push(place);
        }
        place += letter.length;
      }
    }
  });
  cuts.push(located[located.length - 1][1]);
  const last = cuts.length - 1;
  // For each cut some reading reaches, the cuts where a word listed that
  // starts there ends; the cuts reached by as many words are asked about
  // together.
  const ends = cuts.map(() => []);
  const reached = new Set([0]);
  let reaching = [0];
  while (reaching.length) {
    const asking = [];
    for (const cut of reaching) {
      for (let to = cut + 1; to <= last; to += 1) {
        const piece = text.slice(cuts[cut], cuts[to]);
        if ([...piece].length > LONGEST) {
          break;
        }
        const word = fold(piece);
        asking.push(
          (async () => {
            if ((await listed(word)) || (typing && to === last && (await starts(word)))) {
              ends[cut].push(to);
            }
          })(),
        );
      }
    }
    await Promise.all(asking);
    const next = reaching.flatMap((cut) => ends[cut]).filter((to) => !reached.has(to));
    reaching = [...new Set(next)];
    reaching.forEach((cut) => reached.add(cut));
  }
  // From each cut, the fewest words reading the rest, and where the first
  // of them ends; of as many, the one whose first word ends later.
  const best = cuts.map(() => null);
  best[last] = [0, last];
  for (let cut = last - 1; cut >= 0; cut -= 1) {
    for (const to of ends[cut]) {
      if (!best[to]) {
        continue;
      }
      const count = best[to][0] + 1;
      if (!best[cut] || count < best[cut][0] || (count === best[cut][0] && to > best[cut][1])) {
        best[cut] = [count, to];
      }
    }
  }
  if (!best[0]) {
    return null;
  }
  const read = [];
  for (let cut = 0; cut < last; cut = best[cut][1]) {
    read.push(fold(text.slice(cuts[cut], cuts[best[cut][1]])));
  }
  return read;
}

/**
 * The words at `located` in `query`, folded as `all` holds them, each run
 * of words of Chinese, Japanese or Korean next to each other read as
 * `readRun` reads it, the run ending the query typed where `typing`.
 */
async function wordsOf(query, located, all, typing) {
  const unspaced = ([start, end]) => [...query.slice(start, end)].every((c) => UNSPACED.test(c));
  // Each word, or run of words, by where it starts and ends among them.
  const parts = [];
  located.forEach((span, at) => {
    const part = parts[parts.length - 1];
    if (part?.run && unspaced(span) && located[at - 1][1] === span[0]) {
      part.end = at + 1;
    } else {
      parts.push({ start: at, end: at + 1, run: unspaced(span) });
    }
  });
  const read = await Promise.all(
    parts.map(({ start, end, run }) =>
      run ? readRun(query, located.slice(start, end), typing && end === located.length) : null,
    ),
  );
  return parts.flatMap(({ start, end }, at) => read[at] ?? all.slice(start, end));
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

/** Edits between `a` and `b` by code point, a swap of neighbours counting as one. */
function edits(a, b) {
  const [x, y] = [[...a], [...b]];
  let before = [];
  let last = Array.from({ length: y.length + 1 }, (_, j) => j);
  for (let i = 1; i <= x.length; i += 1) {
    const row = [i];
    for (let j = 1; j <= y.length; j += 1) {
      let best = Math.min(last[j] + 1, row[j - 1] + 1, last[j - 1] + (x[i - 1] === y[j - 1] ? 0 : 1));
      if (i > 1 && j > 1 && x[i - 1] === y[j - 2] && x[i - 2] === y[j - 1]) {
        best = Math.min(best, before[j - 2] + 1);
      }
      row.push(best);
    }
    [before, last] = [last, row];
  }
  return last[y.length];
}

/**
 * The records of an entry's list, each as its place among the records,
 * its weight in its text and its weight everywhere else.
 */
function postings(list) {
  const held = [];
  let record = 0;
  for (let at = 0; at < list.length; ) {
    const step = list[at];
    const kind = step % 4;
    record += (step - kind) / 4;
    const text = kind === 2 ? 0 : list[at + 1];
    const rest = kind === 1 ? 0 : list[at + (kind === 2 ? 1 : 2)];
    held.push([record, text, rest]);
    at += kind === 3 ? 3 : 2;
  }
  return held;
}

/**
 * The word nearest `word` in the piece of the word list it falls in: the
 * fewest edits away, then the one held by the most records, then the
 * first. A word of fewer than five characters is near a word one edit
 * away, a longer one two.
 */
async function nearest(word) {
  const length = [...word].length;
  const reach = length < TWO_EDITS ? 1 : 2;
  const piece = await unpacked(FILES.words[pieceOf(word)][1]);
  let found = null;
  piece.words.forEach((listed, place) => {
    if (listed === word || Math.abs([...listed].length - length) > reach) {
      return;
    }
    const apart = edits(word, listed);
    const held = postings(piece.records[piece.stems[place]]).length;
    const nearer =
      !found || apart < found.apart || (apart === found.apart && held > found.held);
    if (apart <= reach && nearer) {
      found = { listed, apart, held };
    }
  });
  return found?.listed ?? null;
}

/**
 * `query` with each word of `asked` that `looked` finds in no records
 * list respelt as the nearest word; none where some such word has none near,
 * or `query` respelt still finds nothing. A word of fewer than three
 * characters, or of Chinese, Japanese or Korean, is kept as written.
 */
async function meant(query, located, all, asked, looked) {
  const respelt = new Map();
  for (const [at, word] of asked.entries()) {
    if (looked[at].length || [...word].length < SHORTEST || UNSPACED.test(word)) {
      continue;
    }
    const near = await nearest(word);
    if (near === null) {
      return null;
    }
    respelt.set(word, near);
  }
  let written = 0;
  let words = "";
  located.forEach(([start, end], at) => {
    if (respelt.has(all[at])) {
      words += query.slice(written, start) + respelt.get(all[at]);
      written = end;
    }
  });
  words += query.slice(written);
  return (await found(words, 0, false, false)).count ? words : null;
}

/**
 * Where each word of `text` is, each run of Chinese, Japanese or Korean
 * next to each other that together make a word in `marked` taken as that
 * one word, the longest first: the browser may cut such text into smaller
 * words than the index did.
 */
function wordsIn(text, marked) {
  const found = spans(text);
  const unspaced = ([start, end]) => [...text.slice(start, end)].every((c) => UNSPACED.test(c));
  const words = [];
  for (let at = 0; at < found.length; at += 1) {
    let last = at;
    if (unspaced(found[at])) {
      for (let to = at + 1; to < found.length; to += 1) {
        if (found[to][0] !== found[to - 1][1] || !unspaced(found[to])) {
          break;
        }
        const run = text.slice(found[at][0], found[to][1]);
        if ([...run].length > LONGEST) {
          break;
        }
        if (marked.has(fold(run))) {
          last = to;
        }
      }
    }
    words.push([found[at][0], found[last][1]]);
    at = last;
  }
  return words;
}

/** The text around the first word in `text` that is `marked`, or its start, cut at words. */
function excerpt(text, marked) {
  const found = wordsIn(text, marked);
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
 * whether it is `marked` as a word asked for. Gives too `suggestion`:
 * where no page is found, the words as typed with each the index doesn't
 * list respelt as the nearest word it does, where every such word has one
 * near and the words respelt find some page; otherwise null. Words of
 * fewer than three characters, or of Chinese, Japanese or Korean, are kept
 * as typed.
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

async function found(query, shown, typing, respelling = true) {
  const located = spans(query);
  const all = located.map(([start, end]) => fold(query.slice(start, end)));
  const lastly = located[located.length - 1];
  const ending = Boolean(typing && lastly && lastly[1] === query.length);
  const words = await wordsOf(query, located, all, ending);
  const typed = ending ? words[words.length - 1] : null;
  const being =
    typed && ([...typed].length > 1 || UNSPACED.test(typed)) ? typed : null;
  const asked = [...new Set(words)].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  if (!asked.length) {
    return { count: 0, partial: false, results: [], suggestion: null };
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
      const held = postings(list);
      const holding = held.length;
      const rarity = Math.log(1 + (count - holding + 0.5) / (holding + 0.5));
      for (const [record, text, rest] of held) {
        const length = lengths[record] / average;
        const weight = rest + text / (1 - B + B * length);
        const score = (rarity * weight * (K1 + 1)) / (weight + K1);
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
  const suggestion =
    !best.length && respelling ? await meant(query, located, all, asked, looked) : null;
  const partial = best.length > 0 && most < asked.length;
  return { count: best.length, partial, results, suggestion };
}
