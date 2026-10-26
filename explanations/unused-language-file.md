# Unused language file

The site has a file in `languages/`, such as `languages/en-US.yaml`, but
no language of the site reads it, so its words are used nowhere.

## What you'll see {#what-youll-see}

```text
note  Unused language file: no language of the site reads `languages/en-US.yaml`
  Name it for the site's language, as `languages/en-GB.yaml`, or take it out.
  More: https://statac.dev/e/unused-language-file/
```

## Why Statac says so

Each of the site's languages, as `language:` in `statac.yaml` names
them, reads one file in `languages/`: the one named for it, or else one
named for a language it comes from, as `languages/en.yaml` for `en-GB`.
A file no language reads, such as `en-US.yaml` on a site in `en-GB`,
changes nothing, and nothing else would show it, so Statac tells you.

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

with its words in `languages/en-US.yaml`. Renamed
`languages/en-GB.yaml`, the file is read.
