# Unknown code language

A code block gives its language as the first word after the opening
fence, and Statac has no colours for that word. Statac publishes the
block without colour, and its `<code>` element keeps the word in its
class, such as `language-qbasic`. Statac tells of this only on a site
with `code_colours: true`, where it colours code.

## What you'll see {#what-youll-see}

```text
warning  Unknown code language: Statac has no colours for `cosole`
  Did you mean `console`?
  More: https://statac.dev/e/unknown-code-language/
```

## Why Statac warns

Statac is sure what to do, so the build continues. It warns you because
the word may be a typo for a language it does colour. Without the
warning, a block you meant to be coloured would be published plain, and
nothing would tell you.

## How to fix it

If the word is a typo, write the language's name as Statac knows it.
https://statac.dev/writing/#languages lists the names of the common
languages.

If the code is in a language Statac doesn't colour, or isn't code at all,
replace the word with `text`. Statac then shows the block without
colour, and without this warning. If the word is there on purpose, so that your own
stylesheet or script can find it in the class, quieten this warning with
`quiet: [unknown-code-language]` in `statac.yaml`.

If your own script colours all the site's code, take `code_colours: true`
out of `statac.yaml` instead. Statac then colours nothing and has no
warning for any language.

## Example

In `content/post-1.md`:

````markdown
```cosole
$ statac build
```
````

With the language's name as Statac knows it:

````markdown
```console
$ statac build
```
````
