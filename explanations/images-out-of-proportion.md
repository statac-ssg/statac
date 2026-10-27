# Images out of proportion

A stylesheet in `assets/` has a rule for `img` that sets a `max-width` or
a `width` for images, but no rule for `img` in any of the site's
stylesheets sets their height. If the rule makes an image narrower, the
image keeps the height written in the page, so it is shown too tall for
its width.

Statac warns once for the whole site, at the first rule like this.

## What you'll see {#what-youll-see}

```text
warning  Images out of proportion: `img` has a `max-width` and no `height`
  Add `height: auto` to the rule.
  More: https://statac.dev/e/images-out-of-proportion/
```

## Why Statac warns

Statac writes each image's `width` and `height` into the page, so the
page keeps a space of the right size for the image and doesn't jump as
images load. A browser treats those two numbers as the image's size. A
rule that changes only the width keeps the height the page wrote. For
example, an image 1600 wide and 1067 tall, shown in a column 400 wide,
is drawn 400 wide but still 1067 tall.

Adding `height: auto` to the rule leaves the height to the browser, which
works it out from the width. The space kept for the image is then still
the right shape.

Statac reads the stylesheets in `assets/`, including Sass. It doesn't warn
if any rule for `img` sets a `height`, a `block-size` or an
`aspect-ratio`, or if no page shows an image with a width and a height.
It doesn't read styles written in a template's `<style>`. If you set the
height there, quieten the warning by name.

## How to fix it

Add `height: auto` to the rule the warning points at. If the rule uses
`max-inline-size` or `inline-size`, add `block-size: auto` instead, which
means the same thing.

## Example

In `assets/site.css`:

```css
img {
  max-width: 100%;
}
```

With the height left to the browser, the warning goes away:

```css
img {
  max-width: 100%;
  height: auto;
}
```

To stop Statac showing the warning, add this to `statac.yaml`:

```yaml
quiet: [images-out-of-proportion]
```
