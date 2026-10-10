# Same id

Two pages have the same `id:`. A page's id is its permanent identity: it
stays the same when the page's title or address changes. The message
points at the second, and shows where the first was written.

## What you'll see {#what-youll-see}

```text
error  Same id: two pages have the id `k3m9x2`
  Give each page its own id.
  More: https://statac.dev/e/same-id/
```

## Why Statac stops

An id names one page. With two pages giving the same one, anything that
follows a page by its id, such as a feed reader, would take one page for
the other.

## How to fix it

Give each page its own id. A page made from an item in `data/` takes the
item's `id:`, so look there too.

## Example

`data/people.yaml`:

```yaml
- name: Person 1
  id: k3m9x2
- name: Person 2
  id: k3m9x2
```

Give the second its own:

```yaml
- name: Person 1
  id: k3m9x2
- name: Person 2
  id: p7q2r8
```
