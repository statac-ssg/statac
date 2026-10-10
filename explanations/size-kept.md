# Size kept

A text on a card is placed at the top or the bottom, and its size is
written `up to`, as `up to 75`. Statac fits only the texts in the middle
row, at `left`, `centre` or `right`, to the room there; a text at the top
or the bottom is drawn at the size written.

## What you'll see {#what-youll-see}

```text
warning  Size kept: at `bottom`, `up to 36` is drawn at 36
  Write `footer.size: 36`; only a text at `left`, `centre` or `right` is fitted.
  More: https://statac.dev/e/size-kept/
```

## Why Statac warns

Statac is sure what you meant: the size is the largest you would have
it, and it draws it at that size, so the build goes on. It tells you,
because `up to` says you expect it to shrink when a long title needs the
room, and here it never does.

## How to fix it

Write the size without `up to`, or place the text at `left`, `centre` or
`right` to have it fitted.

## Example

In `templates/og.yaml`:

```yaml
footer.text: "{{ site.title }}"
footer.place: bottom
footer.size: up to 36
footer.colour: "#B9A88F"
```

Drawn at 36 either way, written as it is drawn:

```yaml
footer.size: 36
```
