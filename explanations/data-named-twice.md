# Data named twice

Two things in `data/` would have the same name in templates. They are
either two files with the same name and different endings, such as
`people.yaml` and `people.json`, or a file and a folder, such as
`people.yaml` and `people/`.

## What you'll see {#what-youll-see}

```text
error  Data named twice: `data/people.json` and `data/people.yaml` are both `site.data.people`
  Rename one of them.
  More: https://statac.dev/e/data-named-twice/
```

## Why Statac stops

Templates read `data/people.yaml` as `site.data.people`, and read a
folder `data/people/` under the same name. If two things share a name, Statac
can't tell which one you meant, and using either would quietly hide the
other.

## How to fix it

Rename one of them, or remove the one you no longer use.

## Example

`data/` contains:

```text
data/people.yaml
data/people.json
```

Keep one, or give the other a name of its own:

```text
data/people.yaml
data/people-old.json
```
