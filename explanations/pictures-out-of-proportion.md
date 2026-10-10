# Pictures out of proportion

A stylesheet in `assets/` has a rule for `img` that gives pictures a
`max-width` or a `width`, and no rule for `img` in any of the site's
stylesheets says what their height is. Where the rule makes a picture
narrower, the picture keeps the height the page gives it, so it is shown
too tall for its width.

It is told once for the site, at the first such rule.

## What you'll see {#what-youll-see}

```text
warning  Pictures out of proportion: `img` has a `max-width` and no `height`
  Add `height: auto` to the rule.
  More: https://statac.dev/e/pictures-out-of-proportion/
```

## Why Statac warns

Statac writes each picture's `width` and `height` into the page, so the
page keeps a place of the right size for the picture and doesn't jump as
pictures load. A browser takes those two numbers as the picture's size.
A rule that changes the width alone leaves the height as the page wrote
it: a photograph 1600 wide and 1067 tall, shown in a column 400 wide, is
drawn 400 wide and still 1067 tall.

`height: auto` in the rule hands the height back to the browser, which
works it out from the width, and the place kept for the picture is still
the right shape.

Statac reads the stylesheets in `assets/`, Sass among them. It says
nothing where any rule for `img` gives a `height`, a `block-size` or an
`aspect-ratio`, or where no page shows a picture with a width and a
height. Styles written in a template's `<style>` are not read: where the
height is given there, quieten the warning by name.

## How to fix it

Add `height: auto` to the rule the warning points at. In a rule written
with `max-inline-size` or `inline-size`, add `block-size: auto`, which
means the same.

## Example

In `assets/site.css`:

```css
img {
  max-width: 100%;
}
```

With the height left to the browser, the warning goes:

```css
img {
  max-width: 100%;
  height: auto;
}
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [pictures-out-of-proportion]
```
