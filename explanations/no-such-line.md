# No such line

A code block's first line asks Statac to mark a line that the block
doesn't have, as `mark=15` does on a block of 12 lines. Or a template
asks `file.code` for a line that isn't there: `from` or `to` past the
file's last line, or a line to mark that isn't among the lines shown.

## What you'll see {#what-youll-see}

```text
warning  No such line: this block has no line 4
  content/post-1.md, line 1
   1 │ ```rust mark=4
     │         ^^^^^^
  The block's last line is 3.
  More: https://statac.dev/e/no-such-line/
```

## Why Statac warns

The block is still worth publishing, so Statac publishes it. It marks
the lines the block has and leaves out the numbers it hasn't. For a
file, it shows the lines up to the file's last line. A number past the
end usually means the code changed after the number was written, so the
lines Statac marks may no longer be the ones you meant.

## How to fix it

Count the lines again and correct the number. A block's first line of
code, the one after its opening fence, is line 1. A file's lines keep
their own numbers when `from` leaves some out: with `from=10`, the first
line shown is line 10, and `mark` names lines by those numbers.

## Example

In `content/post-1.md`:

````markdown
```rust mark=4
fn main() {
    println!("Hello");
}
```
````

The block has three lines, and the line to mark is the second:

````markdown
```rust mark=2
fn main() {
    println!("Hello");
}
```
````
