# Malformed maths

Statac found maths in your writing, between dollar signs, but couldn't
make it into maths for the page. The message says what it found and points
at the place to fix, such as a brace that is never closed or a command
Statac doesn't know.

## What you'll see {#what-youll-see}

```text
error  Malformed maths: Statac doesn't know `\fracc`
  More: https://statac.dev/e/malformed-maths/
```

## Why Statac stops

Statac can't tell what maths it can't read was meant to say: a command
spelt wrong, a brace left out, or dollar signs that were never maths at
all. Publishing what you typed would put the maths' source in front of
readers instead, so Statac stops rather than guess.

## How to fix it

Look at the place the caret points to. Common causes:

- A command Statac doesn't know, often one spelt wrong: `\fracc` for
  `\frac`. [Writing](/writing/#maths) lists what Statac's maths covers.
- A `{` with no `}` to close it, or a `\begin{cases}` with no
  `\end{cases}`.
- An `\end{...}` naming another environment than the `\begin{...}` it
  closes.
- A command with less after it than it takes, such as `\frac{1}` with no
  second part.
- Two `^` or two `_` on one thing, `x^a^b`. Put braces round what goes
  together: `x^{a^b}`.
- An `\eqref` to a label no equation on the page has.

Where the dollar signs were never maths, write a backslash before one:
`\$5`.

## Example

In `content/post-1.md`:

```markdown
The area is $\frac{1}{2 b h$.
```

The `{` before `2` is never closed. Close it with `}`:

```markdown
The area is $\frac{1}{2} b h$.
```
