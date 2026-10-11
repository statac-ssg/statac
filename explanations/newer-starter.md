# Newer starter files

Some of the files that `statac new` wrote when the site was made have a
newer version in this version of Statac. Each is either a file you never
changed, or a file you changed, starting from a version of the starter
that has changed since.

## What you'll see {#what-youll-see}

```text
note  Newer starter files: 1 of the files `statac new` wrote has a newer version
  Run `statac new --update`: it replaces those you never changed and shows what is new in the rest.
  More: https://statac.dev/e/newer-starter/
```

## Why Statac says so

Once `statac new` has written a site's templates, snippets and
stylesheet, they are yours. A later version of Statac never changes them
unless you ask. Statac knows every version each starter file has had, so
it can recognise a file you never changed from its exact bytes, and
nothing extra is kept in the site to record it. A file you changed is
compared with the starter version nearest to it, the one with the fewest
lines different. If that is the newest version, there is nothing new, and
the file isn't counted. Statac shows this note once, in `statac check`
and in the count after `statac build`.

## How to fix it

Run `statac new --update` in the site's folder. It replaces each file you
never changed with the newest version. For each file you changed, it
shows what is new, but never writes to the file. Then copy whatever you
want from that into your own files.

If you want to keep the files as they are, quieten the note.

## Example

`statac new --update` reports what it did:

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
