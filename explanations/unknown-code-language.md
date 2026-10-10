# Unknown code language

A block of code names its language with the first word after the
opening fence, and Statac has no colours for the word written. The block
is published as it is, without colour, and its `<code>` keeps the word in
its class, such as `language-qbasic`.

## What you'll see {#what-youll-see}

```text
warning  Unknown code language: Statac has no colours for `cosole`
  Did you mean `console`?
  More: https://statac.dev/e/unknown-code-language/
```

## Why Statac warns

Statac is sure what to do, so the build goes on. It tells you, because
the word may be a slip for a language it does colour, and a block you
meant to be coloured would otherwise go out plain without a word.

## How to fix it

Where the word is a slip, write the language's name as Statac knows it.
The writing page lists the names of the common languages.

Where the code is in a language Statac doesn't colour, or isn't code at
all, write `text` in place of the word, and the block is shown as it is
without this warning. Where the word is there on purpose, for your own
stylesheet or script to find in the class, quieten this warning with
`quiet: [unknown-code-language]` in `statac.yaml`.

## Example

In `content/post-1.md`:

````markdown
```cosole
$ statac build
```
````

The language as Statac knows it:

````markdown
```console
$ statac build
```
````
