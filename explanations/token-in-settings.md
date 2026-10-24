# Token in settings

`statac.yaml` has a `webmention_token:` line. The token is the key the
webmention service gave you to read your site's webmentions. It doesn't
belong in a file that is kept with the site.

## What you'll see {#what-youll-see}

```text
error  Token in settings: `webmention_token` doesn't belong in a file kept with the site
  Remove the line, add `webmentions: true`, and put the token in `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`.
  More: https://statac.dev/e/token-in-settings/
```

## Why Statac stops

The site's folder is committed, shared and often published, and anyone
who has the token can read every webmention the service stores for you.
So when `statac refresh` asks the webmention service for webmentions,
Statac reads the token from only two places: the variable
`STATAC_WEBMENTION_TOKEN`, or else the single line of the file
`.statac/token`. A new site's `.gitignore` keeps
that folder out of commits. If both have a token, the variable's is used.
A token line in `statac.yaml` stops the build, so the token goes no
further than it already has. Its value is never shown: wherever Statac
prints the line, it prints dots in place of the token.

`statac build` and `statac check` never need the token at all.

## How to fix it

Remove the line from `statac.yaml`, and make sure the token isn't in any of
the files you commit. If it has been committed, ask the webmention
service for a new token, since the old one is in the folder's history. Then say that the
site uses webmentions by adding `webmentions: true`, and give the token to
`statac refresh` in the variable `STATAC_WEBMENTION_TOKEN`:

```sh
STATAC_WEBMENTION_TOKEN=your-token statac refresh
```

Or keep it on this computer, as the one line of the file `.statac/token`
in the site's folder, and run `statac refresh` without the variable. First
check that `.gitignore` lists `.statac/`, so the file is never committed.

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
