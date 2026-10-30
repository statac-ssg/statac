# Broken external link

A link points at a page on another site, and when `statac refresh`
last asked that site for the page, the answer showed the link is
broken. A reader who follows it lands on an error.

## What you'll see {#what-youll-see}

```text
warning  Broken external link: `https://example.org/post-2/` answered 404 Not Found
  content/post-1.md, line 1
   1 │ Read [Post 2](https://example.org/post-2/) next.
     │               ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  `statac refresh` checked it 3 days ago. Change or remove the link.
  More: https://statac.dev/e/broken-external-link/
```

## Why Statac warns

A build never uses the network, so `statac check` can't ask another site
about a link. If `statac.yaml` has `check_external_links: true`, each
`statac refresh` asks the sites your pages link to about the links that
are due, and records each answer, with the day, in `fetched/links`.
`statac check` then reads that record.

A link is broken when the other site answers 404 Not Found or 410 Gone,
when no site has the host's name, or when the host's certificate has
expired or is for another host, so browsers show a warning in place of
the page. Statac warns once for each place the link is written, even if
several pages show it. If you wrote the link in your content, a field or
a data file, the warning shows that line. If a template or snippet wrote
it, the warning shows the line in the template.

Other answers don't say whether the link works: a site that doesn't
answer in time, asks Statac to wait, refuses robots, or shows a page
asking the visitor to prove they are a person. `statac check` counts
those links without listing them.

## How to fix it

Change the link to where the page is now, or remove it. If the page has
gone for good, a copy in a web archive may be the best link to give.

If you keep the link on purpose, list its URL, or a URL above it, under
`unchecked_external_links` in `statac.yaml`. `statac refresh` then
doesn't ask about it, and `statac check` counts it instead of warning.

The answer may be out of date: a site that was down may be back. A
broken answer holds for a week, and the next `statac refresh` after
that asks again.

Once you have understood the warning, you can quieten it by name.

## Example

In `content/post-1.md`:

```markdown
Read [Post 2](https://example.org/post-2/) next.
```

`fetched/links`, as `statac refresh` wrote it:

```text
statac links 1
https://example.org/post-2/	404	2026-10-06
```

To stop checking every link to `example.org`, add this to `statac.yaml`:

```yaml
unchecked_external_links:
  - https://example.org/
```

To stop Statac showing the warning, add this to `statac.yaml`:

```yaml
quiet: [broken-external-link]
```
