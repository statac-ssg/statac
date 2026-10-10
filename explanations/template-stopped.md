# Template stopped

A template stopped while Statac built a page with it, for a reason other
than a missing value. The message says why, such as adding a number to
text, a filter handed something it can't work on, a snippet that calls
itself, or a `!!binary` value whose bytes aren't text shown on the page,
and names the page being built.

## What you'll see {#what-youll-see}

```text
error  Template stopped: `absolute` needs the site's `url`
  Add `url:` to `statac.yaml`.
  More: https://statac.dev/e/template-stopped/
```

## Why Statac stops

The template can't finish the page, and publishing part of it would leave a
broken page in the site.

## How to fix it

Look at the place the caret points to, on the page the message names.
Common causes:

- `absolute` on a site whose `statac.yaml` doesn't give the site's own
  address, which it needs to make a full one.
- `date` or `moment` handed something that isn't a date.
- `headline` handed something that isn't a page, such as a title.
- `url` handed something that isn't an address from the top of the site,
  starting with one `/`.
- A snippet that calls a snippet that calls it back. Take the call out of
  one of them.
- A loop that never ends, or a loop inside a loop over long lists, which
  took all the steps Statac lets one page take. The message names the
  layout the page is built with; the caret points to where the steps ran
  out, which may be in a snippet the layout calls.

## Example

In `templates/post.html`:

```html
<p>{{ page.title | date }}</p>
```

`date` works on a date:

```html
<p>{{ page.date | date }}</p>
```
