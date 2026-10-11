# Not data

A file in `data/` is neither YAML nor JSON. Statac reads only the files
there that end in `.yaml`, `.yml` or `.json`, and leaves out anything
else.

## What you'll see {#what-youll-see}

```text
warning  Not data: `data/people.txt` isn't YAML or JSON
  Save it as `.yaml` or `.json`, or move it out of `data/`.
  More: https://statac.dev/e/not-data/
```

## Why Statac warns

Templates read the files in `data/` through `site.data`, and a file
Statac can't read never appears there. Statac carries on without it and
warns you, in case you meant templates to use it.

## How to fix it

Save the file as YAML or JSON with the matching ending, or move it out of
`data/`. A file that should be published unchanged, such as a PDF, belongs in
`static/` or beside the page that links to it.

## Example

`data/people.txt`:

```text
Person 1
Person 2
```

Saved as `data/people.yaml`, it is available to templates as
`site.data.people`:

```yaml
- Person 1
- Person 2
```
