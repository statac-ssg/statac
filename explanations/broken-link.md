# Broken link

A link, or a page's `redirect_to`, points at an address on the site where
the built site has nothing. A reader who follows it lands on a missing
page.

## What you'll see {#what-youll-see}

```text
warning  Broken link: nothing is published at `/about-us/`
  Did you mean `/about/`?
  More: https://statac.dev/e/broken-link/
```

## Why Statac warns

Statac knows every address it publishes, so once the site is built it
follows every link on every page to one on the site, as a browser would,
and tells you when one leads to none of them. A link in your writing is
shown at its line; one a template or snippet wrote is shown at the line
that wrote it, with the page it is on. Statac publishes the page with the
link as you wrote it, because the link may point at something you are
about to add. A redirect is published the same way. Links to other sites
are left alone: a build never fetches anything.

## How to fix it

Point the link at an address the site has. When a published address is
close to the one you wrote, Statac offers it. A link names a page by its
address, not its file: where one names a file under `content/`, such as
`post-2.md`, Statac offers that page's address. If the link is right and the
page is still to come, the warning goes once the page is published. If
something other than Statac publishes there, such as another program on
the same host, name the path in `not_ours: [/path/]` in `statac.yaml`.

On a site published under a path, such as `/blog/`, a link written from
the top of the domain, such as `/about/`, leads outside the site, and
Statac leaves it as written. Where the site publishes `/blog/about/`,
Statac offers the way to link inside the site wherever it is published:
`@url(/about/)` in the writing, `{{ url("/about/") }}` in a template.

## Example

In `content/post-1.md`:

```markdown
Read [Post 2](/post-2/) next.
```

If `content/post-2.md` is published at `/posts/post-2/`, write the link as:

```markdown
Read [Post 2](/posts/post-2/) next.
```
