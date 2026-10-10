# Token in settings

`statac.yaml` has a `webmention_token:` line. The token is the key the
webmention relay gave you to read your site's mentions, and it doesn't
belong in a file kept with the site.

## What you'll see {#what-youll-see}

```text
error  Token in settings: `webmention_token` doesn't belong in a file kept with the site
  Remove the line, add `webmentions: true`, and put the token in `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`.
  More: https://statac.dev/e/token-in-settings/
```

## Why Statac stops

The site's folder is committed, shared and often published, and whoever
holds the token can read every mention the relay keeps for you. So Statac
reads the token from two places only, when `statac refresh` asks the relay:
the variable `STATAC_WEBMENTION_TOKEN`, or else the one line of the file
`.statac/token`, in a folder a new site's `.gitignore` keeps out of what is
committed. Where both hold one, the variable's is used. A line holding it in
`statac.yaml` stops the build, so the token goes no further than it has, and
its value is never shown: wherever Statac draws the line, it draws dots in
its place.

`statac build` and `statac check` never need the token at all.

## How to fix it

Remove the line from `statac.yaml`, and make sure the token is in none of
the files you commit. If it has been committed, ask the relay for a new
one, since the old one is in the history of the folder. Then say the site
uses webmentions with `webmentions: true`, and give the token to
`statac refresh` in the variable `STATAC_WEBMENTION_TOKEN`:

```sh
STATAC_WEBMENTION_TOKEN=your-token statac refresh
```

Or keep it on this computer, as the one line of the file `.statac/token`
in the site's folder, and run `statac refresh` with no variable. Check
that `.gitignore` lists `.statac/` first, so the file is never committed.

## Example

In `statac.yaml`:

```yaml
url: https://example.org/
webmention_token: •••
```

With the line removed:

```yaml
url: https://example.org/
webmentions: true
```
