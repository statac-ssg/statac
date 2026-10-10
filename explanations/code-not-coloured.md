# Code not coloured

A block of code names a language Statac colours, and one line of it is
longer than 5,000 characters. Statac did not colour the block. It is
published as it is, the way a block in a language Statac has no colours
for is, and its `<code>` keeps the word in its class, such as
`language-js`.

## What you'll see {#what-youll-see}

```text
warning  Code not coloured: a line in this block is 52,310 characters long
  Break the line up, or take the language off the block's first line.
  More: https://statac.dev/e/code-not-coloured/
```

## Why Statac warns

The time colouring takes grows with the square of a line's length: a line
of 50,000 characters takes seconds, and one of 200,000 takes minutes, on
every build. A line that long is nearly always code one program wrote for
another to read, such as a script with its line breaks taken out, where
colour helps no reader. So Statac leaves the block without colour and the
build stays quick. It tells you, because a block you meant to be coloured
would otherwise go out plain without a word.

The length is counted in characters, and a line of exactly 5,000 is still
coloured.

## How to fix it

Break the long line into shorter ones, where the code means the same with
the breaks in.

Where the code has to stay on one line, take the language off the block's
first line, or write `text` in its place. The block is shown as it is,
which is how Statac publishes it already, and the warning goes. To keep
the word for its class, quieten this warning with
`quiet: [code-not-coloured]` in `statac.yaml`.

## Example

In `content/post-1.md`, where the block's one line of code is 52,310
characters long, shortened here:

````markdown
```js
!function(){var a=1,b=2;a+=b}();
```
````

With no language named, the block is shown as it is and nothing is said:

````markdown
```
!function(){var a=1,b=2;a+=b}();
```
````
