# Output folder in the way

The output folder given to `statac build` with `--out` is one of the
folders Statac reads the site from, or is inside one: `content/`,
`templates/`, `snippets/`, `assets/`, `static/`, `data/`, `languages/`
or `fetched/`.

## What you'll see {#what-youll-see}

```text
error  Output folder in the way: `content/built` is inside `content/`
  Build into a folder Statac doesn't read, such as `--out=_site`.
  More: https://statac.dev/e/output-in-the-way/
```

## Why Statac stops

The next build would read the built site as part of the site and publish
it again, one folder deeper each time. Statac stops before it runs
`before_build` or writes anything.

## How to fix it

Build into a folder Statac doesn't read. Without `--out`, Statac builds
into `_site` in the site's folder. Any other folder outside those eight,
such as `public`, works as well.

## Example

```text
statac build --out=content/built
```

`content/built` is inside `content/`, where the site's writing is. To
build into `public` instead:

```text
statac build --out=public
```
