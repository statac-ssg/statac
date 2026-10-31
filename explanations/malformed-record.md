# Malformed record

A line in `fetched/record`, `fetched/links` or `published.txt` isn't in
the form Statac writes. `fetched/record` lists what `statac refresh`
fetched into `fetched/`, and when. `fetched/links` lists what other
sites answered when `statac refresh` checked the links to them.
`published.txt` lists every URL the site has published.

## What you'll see {#what-youll-see}

```text
error  Malformed record: `6 October` isn't a day
  fetched/record, line 2
   2 │ stills/youtube-abcdefghijk.jpg  0f3a9c1e5b7d2468    6 October
     │                                                     ^^^^^^^^^
  Write the day as `2026-10-06`.
  More: https://statac.dev/e/malformed-record/
```

## Why Statac stops

Statac writes these files itself, and reads them back each time it
builds the site. A line written any other way, usually by hand or by a
merge that went wrong, isn't something Statac can rely on, so the build
stops at that line rather than guess.

`statac refresh` writes `fetched/record`, and `statac build` reads it to
report how old the fetched files are. Its first line is `statac fetched
1`. Each line after that describes one file, in three parts separated by
tabs:

- the file's path under `fetched/`, such as
  `stills/youtube-abcdefghijk.jpg`
- the first 16 hex digits of the BLAKE3 hash of the file, or `absent`
  if there is no file: the other site replied that it has none, or
  `statac refresh` chose not to keep it, such as for a refused webmention
- the date it was fetched, such as `2026-10-06`

A malformed line in `fetched/record` usually has a part missing, spaces
where a tab should be, or a date written another way, such as
`6 October`.

`statac refresh` writes `fetched/links`, and `statac check` reads it to
tell you which links to other sites are broken or have moved. Its first
line is `statac links 1`. Each line after that describes one link, in
three parts separated by tabs:

- the full URL the link points at, without any `#` part, such as
  `https://example.org/post-2/`
- the answer: `ok`; `moved` and the URL it redirects to permanently;
  `404` or `410`; `no-host`; `certificate`; or `unanswered` and why,
  such as `unanswered 403` or `unanswered timeout`
- the date it was checked, such as `2026-10-06`

A malformed line in `fetched/links` usually has a URL that isn't a full
URL, such as `example.org/post-2/`, or an answer Statac doesn't record,
such as `broken`.

`published.txt` is at the top of the site's folder. Each time it builds
the site, Statac adds the URLs it published, and warns if a URL in the
file is no longer published. Lines starting with `#` say what the file
is, and Statac skips blank lines. Every other line is one URL:

- starting with `/`, from the top of the site, without the site's own
  path: `/about/` for `https://example.org/blog/about/` if the site's
  `url` is `https://example.org/blog/`
- ending with `/` for a page or a folder, and without it for a file,
  such as `/feed.xml`
- with its characters written as themselves, such as `/café/`, not as
  `%` codes, and with no `#` or `..` in it
- on one line only

A malformed line in `published.txt` is usually a URL without its
opening `/`, such as `about/`, a full URL, such as
`https://example.org/about/`, or a URL written on two lines.

## How to fix it

In `fetched/record` or `fetched/links`, put the line back as Statac wrote
it, which you can find in your history of the file. Or delete the line.
If you delete a line of `fetched/record`, Statac counts the file as your
own, with no fetch date, and if the line marked the file `absent`, the
next `statac refresh` asks for it again. If you delete a line of
`fetched/links`, the next `statac refresh` checks that link again.

In `published.txt`, write the URL the line meant, starting with `/`,
such as `/about/` in place of `about/` or `https://example.org/about/`.
If a URL is on two lines, take one of them out. Or take the line out:
Statac then stops warning if that URL is no longer published. If the
site still publishes the URL, the next build adds the line back, unless
it is one of the list pages Statac makes, such as a tag's, or a
paginated page.

## Example

`fetched/record`, with a line whose date isn't a date:

```text
statac fetched 1
stills/youtube-abcdefghijk.jpg	0f3a9c1e5b7d2468	6 October
```

With the date written the way Statac writes it:

```text
statac fetched 1
stills/youtube-abcdefghijk.jpg	0f3a9c1e5b7d2468	2026-10-06
```

`published.txt`, with a URL missing its opening `/`:

```text
# Every URL this site has published, so Statac can say when one is no
# longer published. Statac adds to it each time it builds the site.
# Take out the line of a URL that is gone for good.
/
about/
```

With the URL written the way Statac writes it:

```text
# Every URL this site has published, so Statac can say when one is no
# longer published. Statac adds to it each time it builds the site.
# Take out the line of a URL that is gone for good.
/
/about/
```
