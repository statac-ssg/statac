# No site here

A command that works on a site, such as `statac build`, found no site to
work on. A site is a folder with a `statac.yaml` (the site's settings) or a
`content/` folder (the site's writing).

If you named a folder, after the command or with `--site`, Statac looks
only in that folder, and it has neither. If you didn't, Statac looks in
the current folder and each folder above it, and none of them has
either.

## What you'll see {#what-youll-see}

```text
error  No site here: `.` has no `content/` folder and no `statac.yaml`
  Run it in a site's folder, or make one with `statac new`.
  More: https://statac.dev/e/no-site-here/
```

## Why Statac stops

Building a folder that isn't a site would publish an empty site, and
leave Statac's own files in a folder you didn't intend. Statac doesn't
guess which folder you meant.

## How to fix it

Run the command in the site's folder, or in any folder inside it, or name
the site's folder after the command. If there is no site yet, make one
with `statac new`.

## Example

This command fails if `notes` contains only a few Markdown files:

```text
statac build notes
```

To make a new site and build it instead:

```text
statac new site-1
statac build site-1
```
