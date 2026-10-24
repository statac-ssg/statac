# Template stopped

A template failed while Statac was building a page with it, for a reason
other than a missing value. The message gives the reason and the page
being built. Reasons include adding a number to text, giving a filter
something it can't work on, a snippet that calls itself, and showing a
`!!binary` value on the page when its bytes aren't text.

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

Look at the place the caret points to, on the page the message gives.
Common causes:

- `absolute` used on a site whose `statac.yaml` doesn't give the site's
  full URL. `absolute` needs it to make a full URL.
- `date` or `moment` given something that isn't a date.
- `headline` given something that isn't a page, such as a title.
- `url` given something that isn't a URL starting with a single `/`.
- A snippet that calls another snippet, which calls the first one back.
  Remove the call from one of them.
- A loop that never ends, or a loop inside a loop over long lists, which
  used up all the steps Statac allows for one page. The message gives the
  layout the page is built with. The caret points to where the steps ran
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
