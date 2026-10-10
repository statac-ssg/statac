# Webmentions off

A template calls `webmention()`, which tells other sites where to send
the mentions of your pages, and `statac.yaml` doesn't say
`webmentions: true`. The call writes nothing.

## What you'll see {#what-youll-see}

```text
warning  Webmentions off: `webmention()` writes nothing here
  Add `webmentions: true` to `statac.yaml`, or remove the call.
  More: https://statac.dev/e/webmentions-off/
```

## Why Statac warns

`webmention()` writes the line in a page's `<head>` that names the relay
receiving mentions for the site. Another site that links to the page
reads the line and tells the relay. Without `webmentions: true`, Statac
doesn't ask the relay for what it receives, so the line would ask other
sites to send mentions that nothing reads, and Statac leaves it out.

## How to fix it

Add `webmentions: true` to `statac.yaml` if the site uses webmentions,
and run `statac refresh` to fetch them. Otherwise remove the call from
the template.

Once you have understood it, the warning can be quietened by name.

## Example

`templates/page.html`, on a site without `webmentions: true`:

```html
<head>
  <title>{{ page.title }}</title>
  {{ webmention() }}
</head>
```

In `statac.yaml`, to have the call write its line:

```yaml
url: https://example.org/
webmentions: true
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [webmentions-off]
```
