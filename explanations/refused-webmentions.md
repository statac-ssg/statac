# Refused webmentions

`refuse_webmentions_from` in `statac.yaml` refuses some of the
webmentions in `fetched/mentions/`, so no page shows them. Their files
stay in `fetched/mentions/` until `statac refresh` deletes them.

## What you'll see {#what-youll-see}

```text
note  Refused webmentions: `refuse_webmentions_from` refuses 2 in `fetched/mentions/`
  Run `statac refresh` to delete their files.
  More: https://statac.dev/e/refused-webmentions/
```

## Why Statac says so

`refuse_webmentions_from` lists URLs, one a line. It refuses a webmention
when the page that sent it is at or under one of them, or when its
sender's own URL is one of them. A build leaves a refused webmention out as soon as
its URL is listed, whether or not its file is still there. Statac counts
the ones whose files are still there, which are still in the site's
history and in the folder you publish from.

`statac refresh` deletes the file of each refused webmention, and each
avatar in `fetched/faces/` that no other webmention uses. It records each
webmention it deletes in `fetched/record`, so it is never fetched again,
and it doesn't keep a new webmention the list refuses.

## How to fix it

Run `statac refresh`, then commit `fetched/` with the site. The note goes
once the files are deleted.

Once `statac refresh` has deleted a webmention, taking its URL out of the
list doesn't bring it back. `statac refresh` asks the webmention service
only for webmentions newer than the newest it holds, and never fetches
one it has recorded. To undo a refusal, put the files and their lines in
`fetched/record` back from the site's history.

Once you have understood the note, you can quieten it by name.

## Example

In `statac.yaml`:

```yaml
refuse_webmentions_from:
  - https://a.example/post-1/
  - https://b.example/
```

The first line refuses the webmention sent from that page, and any sent
from a page under it. The second refuses everything sent from
`b.example`, and everything from a sender whose own URL is there.

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [refused-webmentions]
```
