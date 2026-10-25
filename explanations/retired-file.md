# Retired file

The site still has `fetched/refused`, which Statac no longer reads. It
listed the webmentions turned away by `statac refuse`, a command Statac
no longer has. The URLs to refuse are listed in `statac.yaml` now, as
`refuse_webmentions_from`.

## What you'll see {#what-youll-see}

```text
error  Retired file: Statac no longer reads `fetched/refused`
  Delete `fetched/refused` after adding them.
  More: https://statac.dev/e/retired-file/
```

## Why Statac stops

Statac no longer refuses what the file lists. Left alone, `statac
refresh` would keep new webmentions from the pages and senders in it, so
Statac stops until they are listed in `statac.yaml`.

`statac check` gives the lines of `statac.yaml` that carry the file's URLs
over, between the headline and the suggestion. A line of the file that
doesn't read as Statac wrote it, usually because it was changed by hand,
has no URL among them, and the suggestion says how many such lines there
are.

## How to fix it

Add the lines `statac check` gives to `statac.yaml`, then delete
`fetched/refused`. Statac never deletes it.

A URL in `refuse_webmentions_from` refuses everything sent from a page at
or under it, and everything from a sender whose own URL is at or under
it. A `mention` line of the file refused only the page at its URL, so the
same URL in `refuse_webmentions_from` also refuses webmentions sent from
pages under it.

## Example

What `statac check` says of a site whose `fetched/refused` lists two
URLs:

```text
error  Retired file: Statac no longer reads `fetched/refused`
  To keep refusing what it lists, add these lines to `statac.yaml`:
    refuse_webmentions_from:
      - https://a.example/post-1/
      - https://b.example/
  Delete `fetched/refused` after adding them.
  More: https://statac.dev/e/retired-file/
```

`statac.yaml`, with the lines added:

```yaml
title: Site
url: https://example.org/
webmentions: true
refuse_webmentions_from:
  - https://a.example/post-1/
  - https://b.example/
```
