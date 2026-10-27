# Unused language file

The site has a file in `languages/`, such as `languages/en-US.yaml`, but
Statac reads it for none of the site's languages, so nothing in it is
used.

## What you'll see {#what-youll-see}

```text
note  Unused language file: Statac reads `languages/en-US.yaml` for none of the site's languages
  Name it for the site's language, `languages/en-GB.yaml`, or take it out.
  More: https://statac.dev/e/unused-language-file/
```

## Why Statac says so

For each language that `language:` in `statac.yaml` names, Statac reads
one file in `languages/`: the file named for it, or if there is none,
the file named for a language it comes from, such as `languages/en.yaml`
for `en-GB`. A file no language reads, such as `en-US.yaml` on a site in
`en-GB`, changes nothing, and nothing else would show it, so Statac
tells you.

## How to fix it

Rename the file for one of the site's languages, or for the language it
comes from, such as `languages/en.yaml` for a site in `en-GB`. If the
site names no `language`, write one in `statac.yaml`. If you don't need
the file, take it out, or quieten the note with
`quiet: [unused-language-file]` in `statac.yaml`.

## Example

A site in British English:

```yaml
language: en-GB
```

with its translations in `languages/en-US.yaml`. Rename the file
`languages/en-GB.yaml`, and Statac reads it.
