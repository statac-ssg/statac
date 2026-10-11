# Duplicate key

The same key is written twice at one level of some YAML or JSON: twice in
a page's front matter, twice under the same key in a settings or data file,
or twice in one object of a JSON file. The message points at the second
one, and shows where the first was written.

## What you'll see {#what-youll-see}

```text
error  Duplicate key: `title`
  More: https://statac.dev/e/duplicate-key/
```

## Why Statac stops

YAML doesn't allow the same key twice, and JSON leaves each program that
reads it to decide. Statac can't know which of the two values you meant,
and taking either one would quietly drop the other.

## How to fix it

Keep the value you want and remove the other line.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: 2026-10-04
title: Post 2
---
```

Keep one `title`:

```yaml
---
title: Post 1
date: 2026-10-04
---
```
