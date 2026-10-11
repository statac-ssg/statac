# Malformed maths

Statac found maths between dollar signs in your writing, but couldn't
turn it into maths on the page. The message says what it found and points
at the place to fix, such as a brace that is never closed or a command
Statac doesn't know.

## What you'll see {#what-youll-see}

```text
error  Malformed maths: Statac doesn't know `\fracc`
  More: https://statac.dev/e/malformed-maths/
```

## Why Statac stops

If Statac can't read some maths, it can't tell what the maths was meant
to say. A command may be spelt wrong, a brace may be missing, or the
dollar signs may never have been maths at all. Publishing what you typed
would show readers the maths' source instead, so Statac stops rather
than guess.

## How to fix it

Look at the place the caret points to. Common causes:

- A command Statac doesn't know, often a misspelt one: `\fracc` for
  `\frac`. [Writing](/writing/#maths) lists the maths Statac supports.
- A `{` with no `}` to close it, or a `\begin{cases}` with no
  `\end{cases}`.
- An `\end{...}` that names a different environment from the
  `\begin{...}` it closes.
- A command followed by fewer parts than it needs, such as `\frac{1}`
  with no second part.
- Two `^` or two `_` on the same thing, as in `x^a^b`. Put braces round
  the parts that belong together: `x^{a^b}`.
- An `\eqref` to a label that no equation on the page has.

If the dollar signs were never meant as maths, put a backslash before
one: `\$5`.

## Example

In `content/post-1.md`:

```markdown
The area is $\frac{1}{2 b h$.
```

The `{` before `2` is never closed. Close it with `}`:

```markdown
The area is $\frac{1}{2} b h$.
```
