# Broken link

A link, or a page's `redirect_to`, points at a URL on this site, but
nothing is published at that URL. A reader who follows it lands on a
missing page.

## What you'll see {#what-youll-see}

```text
warning  Broken link: nothing is published at `/about-us/`
  Did you mean `/about/`?
  More: https://statac.dev/e/broken-link/
```

## Why Statac warns

Statac knows every URL it publishes. Once the site is built, it follows
every link on every page that points to this site, as a browser would, and
tells you if a link leads to a URL with nothing published at it. If you
wrote the link's URL, in your content, a field or a data file, the warning
shows that line, even if a template or snippet prints it. If a template
or snippet wrote the URL itself, the warning shows the line that wrote it,
and the page the link is on. Statac still publishes the page, with the
link as you wrote it, because the link may point at something you are
about to add. It publishes a redirect the same way. Statac doesn't check
links to other sites, because a build never fetches anything.

## How to fix it

Change the link to a URL the site publishes. If a published URL is close
to the one you wrote, Statac suggests it. A link refers to a page by its
URL, not by its file. If a link points at a file under `content/`, such as
`post-2.md`, Statac suggests that page's URL. If the link is right and you
haven't published the page yet, the warning goes away once you do. If
something other than Statac publishes at that URL, such as another program
on the same host, list the path in `served_elsewhere: [/path/]` in
`statac.yaml`.

If the site is published under a path, such as `/blog/`, a link that
starts at the root of the domain, such as `/about/`, leads outside the
site. Statac leaves the link as you wrote it. If the site publishes
`/blog/about/`, Statac suggests a link that stays inside the site wherever
the site is published: `@url(/about/)` in your content, or
`{{ url("/about/") }}` in a template.

## Example

In `content/post-1.md`:

```markdown
Read [Post 2](/post-2/) next.
```

If `content/post-2.md` is published at `/posts/post-2/`, write the link as:

```markdown
Read [Post 2](/posts/post-2/) next.
```
