# Malformed record

A line of `fetched/record` doesn't read as Statac writes it. The record
says what `statac refresh` fetched into `fetched/`, and when.

## What you'll see {#what-youll-see}

```text
error  Malformed record: the first line isn't `statac fetched 1`
  Write `statac fetched 1` as its first line.
  More: https://statac.dev/e/malformed-record/
```

## Why Statac stops

`statac refresh` writes the record, and `statac build` reads it to say
how old what was fetched is. Its first line is `statac fetched 1`. Each
line after it is one file, in three parts parted by tabs:

- the file's path under `fetched/`, such as
  `stills/youtube-abcdefghijk.jpg`
- the first 16 hex digits of the BLAKE3 hash of the file, or `absent`
  where the other site answered that it has no such file
- the day it was fetched, such as `2026-10-06`

`fetched/refused`, the webmentions and senders `statac refuse` turned
away, is written the same way. Its first line is `statac refused 1`,
and each line after it is one address, in three parts parted by tabs:

- `mention`, for the one webmention sent from the address, or `sender`,
  for everything sent from at or under it
- the address, such as `https://a.example/post-1/`
- the day it was refused, such as `2026-10-06`

A line written some other way, mostly by hand or by a merge gone wrong,
says nothing Statac can rely on, so the build stops at it rather than
guess.

## How to fix it

Put the line back as Statac wrote it, as your history of the file holds
it. Or delete the line: Statac then counts the file as your own, with no
day, and a file marked `absent` is asked for again by the next
`statac refresh`.

In `fetched/refused`, put the line back as Statac wrote it, or delete it:
`statac refresh` then keeps what comes from the address again.

## Example

`fetched/record`, with a line whose day isn't one:

```text
statac fetched 1
stills/youtube-abcdefghijk.jpg	0f3a9c1e5b7d2468	6 October
```

With the day as Statac writes it:

```text
statac fetched 1
stills/youtube-abcdefghijk.jpg	0f3a9c1e5b7d2468	2026-10-06
```
