# Malformed record

A line in `fetched/record` isn't in the form Statac writes. The record
lists what `statac refresh` fetched into `fetched/`, and when.

## What you'll see {#what-youll-see}

```text
error  Malformed record: the first line isn't `statac fetched 1`
  Write `statac fetched 1` as its first line.
  More: https://statac.dev/e/malformed-record/
```

## Why Statac stops

`statac refresh` writes the record, and `statac build` reads it to report
how old the fetched files are. Its first line is `statac fetched 1`. Each
line after that describes one file, in three parts separated by tabs:

- the file's path under `fetched/`, such as
  `stills/youtube-abcdefghijk.jpg`
- the first 16 hex digits of the BLAKE3 hash of the file, or `absent`
  if the other site replied that it has no such file
- the date it was fetched, such as `2026-10-06`

`fetched/refused` lists the webmentions and senders that `statac refuse`
turned away, and is written the same way. Its first line is
`statac refused 1`, and each line after that describes one URL, in three
parts separated by tabs:

- `mention`, for the single webmention sent from the URL, or `sender`,
  for everything sent from that URL or any URL under it
- the URL, such as `https://a.example/post-1/`
- the date it was refused, such as `2026-10-06`

A line written any other way, usually by hand or by a merge that went
wrong, isn't something Statac can rely on, so the build stops at that line
rather than guess.

## How to fix it

Put the line back as Statac wrote it, which you can find in your history
of the file. Or delete the line. Statac then counts the file as your own,
with no fetch date. If the line marked the file `absent`, the next
`statac refresh` asks for it again.

In `fetched/refused`, put the line back as Statac wrote it, or delete it.
If you delete it, `statac refresh` keeps what is sent from that URL
again.

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
