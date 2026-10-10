# Not data

A file in `data/` is neither YAML nor JSON. Statac reads files ending in
`.yaml`, `.yml` or `.json` there, and leaves anything else out.

## What you'll see {#what-youll-see}

```text
warning  Not data: `data/people.txt` isn't YAML or JSON
  Save it as `.yaml` or `.json`, or move it out of `data/`.
  More: https://statac.dev/e/not-data/
```

## Why Statac warns

Templates reach the files in `data/` as `site.data`, and a file Statac
can't read would never arrive there. Statac goes on without it and says so,
in case you meant templates to have it.

## How to fix it

Save the file as YAML or JSON with the matching ending, or move it out of
`data/`. A file to be published as it is, such as a PDF, belongs in
`static/` or beside the page that links to it.

## Example

`data/people.txt`:

```text
Person 1
Person 2
```

Saved as `data/people.yaml`, it reaches templates as `site.data.people`:

```yaml
- Person 1
- Person 2
```
