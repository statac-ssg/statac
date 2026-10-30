# Malformed record

A line in `fetched/record` or `fetched/links` isn't in the form Statac
writes. `fetched/record` lists what `statac refresh` fetched into
`fetched/`, and when. `fetched/links` lists what other sites answered
when `statac refresh` checked the links to them.

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

`statac refresh` writes the record, and `statac build` reads it to report
how old the fetched files are. Its first line is `statac fetched 1`. Each
line after that describes one file, in three parts separated by tabs:

- the file's path under `fetched/`, such as
  `stills/youtube-abcdefghijk.jpg`
- the first 16 hex digits of the BLAKE3 hash of the file, or `absent`
  if there is no file: the other site replied that it has none, or
  `statac refresh` chose not to keep it, such as for a refused webmention
- the date it was fetched, such as `2026-10-06`

`fetched/links` starts with `statac links 1`. Each line after that
describes one link, in three parts separated by tabs:

- the full URL the link points at, without any `#` part, such as
  `https://example.org/post-2/`
- the answer: `ok`; `moved` and the URL it redirects to permanently;
  `404` or `410`; `no-host`; `certificate`; or `unanswered` and why,
  such as `unanswered 403` or `unanswered timeout`
- the date it was checked, such as `2026-10-06`

A line written any other way, usually by hand or by a merge that went
wrong, isn't something Statac can rely on, so the build stops at that line
rather than guess.

## How to fix it

Put the line back as Statac wrote it, which you can find in your history
of the file. Or delete the line. Statac then counts the file as your own,
with no fetch date. If the line marked the file `absent`, the next
`statac refresh` asks for it again. If you delete a line of
`fetched/links`, the next `statac refresh` checks that link again.

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
