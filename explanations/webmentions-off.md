# Webmentions off

A template calls `webmention()`, which tells other sites where to send
mentions of your pages, but `statac.yaml` doesn't have
`webmentions: true`. The call writes nothing.

## What you'll see {#what-youll-see}

```text
warning  Webmentions off: `webmention()` writes nothing here
  Add `webmentions: true` to `statac.yaml`, or remove the call.
  More: https://statac.dev/e/webmentions-off/
```

## Why Statac warns

`webmention()` writes a line in the page's `<head>` that gives the
address of the relay that receives mentions for the site. Another site
that links to the page reads this line and notifies the relay. Without
`webmentions: true`, Statac doesn't fetch what the relay has received,
so the line would ask other sites to send mentions that nothing reads.
So Statac leaves the line out.

## How to fix it

Add `webmentions: true` to `statac.yaml` if the site uses webmentions,
and run `statac refresh` to fetch them. Otherwise remove the call from
the template.

Once you have understood it, you can quieten the warning by name.

## Example

In `templates/page.html`, on a site without `webmentions: true`:

```html
<head>
  <title>{{ page.title }}</title>
  {{ webmention() }}
</head>
```

In `statac.yaml`, to make the call write its line:

```yaml
url: https://example.org/
webmentions: true
```

In `statac.yaml`, to stop the warning being shown:

```yaml
quiet: [webmentions-off]
```
