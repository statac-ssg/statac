# Newer starter files

Some of the files `statac new` wrote as the site was made have a newer
version in this Statac. Each is a file you never changed, or a file of
yours made from a version of the starter that has changed since.

## What you'll see {#what-youll-see}

```text
note  Newer starter files: 1 of the files `statac new` wrote has a newer version
  Run `statac new --update`: it replaces those you never changed and shows what is new in the rest.
  More: https://statac.dev/e/newer-starter/
```

## Why Statac says so

Once `statac new` has written a site's templates, snippets and
stylesheet, they are yours, and a better version in a later Statac never
reaches them unasked. Statac knows every version each starter file has
had, so it tells a file you never changed by its bytes, and nothing is
kept in the site to tell it. A file you changed is held to the version
nearest to it, by the fewest lines differing; where that is the newest,
there is nothing new, and it isn't counted. Statac says this once, on
`statac check` and in the count after `statac build`.

## How to fix it

Run `statac new --update` in the site's folder. It puts the newest
version in place of each file you never changed, and shows what is new
in each of yours, which it never writes to. Then copy into your own
files whatever of it you want.

If you mean to keep the files as they are, quieten it.

## Example

`statac new --update` says what it did:

```text
Updated `templates/base.html`.
`templates/post.html` is yours and wasn't touched. Since the starter file nearest to it, the starter's has changed:
  @@ line 12
   <h1>{{ headline(page) }}</h1>
  -<p>{{ page.date }}</p>
  +<p><time datetime="{{ page.date | moment }}">{{ page.date }}</time></p>
   {% if page.tags %}
Updated 1 file. 1 of yours has something new, shown above.
```
