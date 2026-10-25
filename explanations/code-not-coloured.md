# Code not coloured

A code block names a language that Statac colours, and one of its lines
is longer than 5,000 characters. Statac did not colour the block. It is
published as it is, the same as a block in a language Statac has no
colours for, and its `<code>` element keeps the language name in its
class, such as `language-js`. Statac tells of this only on a site with
`code_colours: true`, where it colours code.

## What you'll see {#what-youll-see}

```text
warning  Code not coloured: a line in this block is 52,310 characters long
  Break the line up, or take the language off the block's first line.
  More: https://statac.dev/e/code-not-coloured/
```

## Why Statac warns

The time it takes to colour a line grows with the square of the line's
length. A line of 50,000 characters takes seconds, and one of 200,000
takes minutes, on every build. A line that long is nearly always code
written by one program for another to read, such as a script with its
line breaks removed, and colour helps no reader there. So Statac leaves
the block uncoloured, and the build stays quick. It warns you because
otherwise a block you meant to be coloured would be published plain
without any warning.

The length is counted in characters, and a line of exactly 5,000 is still
coloured.

## How to fix it

Break the long line into shorter ones, if the code still means the same
with the breaks added.

If the code has to stay on one line, remove the language from the
block's first line, or replace it with `text`. The block is shown as it
is, which is how Statac already publishes it, and the warning goes away.
To keep the language name in the class, quieten this warning with
`quiet: [code-not-coloured]` in `statac.yaml`.

## Example

In `content/post-1.md`, the block's one line of code is 52,310
characters long (shortened here):

````markdown
```js
!function(){var a=1,b=2;a+=b}();
```
````

With no language named, the block is shown as it is, and Statac gives no warning:

````markdown
```
!function(){var a=1,b=2;a+=b}();
```
````
