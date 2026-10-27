# Size kept

A text on a card is placed at the top or the bottom, and its size is
written with `up to`, as in `up to 75`. Statac fits only the texts in
the middle row (at `left`, `centre` or `right`) to the space there. A
text at the top or the bottom is drawn at the size written.

## What you'll see {#what-youll-see}

```text
warning  Size kept: at `bottom`, Statac draws `up to 36` at 36
  Write `footer.size: 36`; Statac fits only a text at `left`, `centre` or `right`.
  More: https://statac.dev/e/size-kept/
```

## Why Statac warns

Statac is sure what you meant: the size is the largest you want the
text to be. It draws the text at that size, and the build goes on. It
warns you because `up to` says you expect the text to shrink when a long
title needs the room, and here it never does.

## How to fix it

Write the size without `up to`, or place the text at `left`, `centre` or
`right` so that it is fitted.

## Example

In `templates/og.yaml`:

```yaml
footer.text: "{{ site.title }}"
footer.place: bottom
footer.size: up to 36
footer.colour: "#B9A88F"
```

The text is drawn at 36 either way, so write the size it is drawn at:

```yaml
footer.size: 36
```
