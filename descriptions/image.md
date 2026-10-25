# An image in a template

What a template reads on an image `page.image` gives, and on each image
`page.images` gives, a video's `thumbnail`, a mention's `avatar` and
`image` in `image.html`. Shown as it is, an image is its `src`. A custom
field in the image's `.yaml` file is read the same way, as
`image.credit`.

## `src`

The image's URL, for the `src` of an `<img>`: its widest resized copy no
more than 1200 pixels wide, or the image itself where it has no copies.

More: https://statac.dev/templates/#files

## `avif`

The image's AVIF copies, written as a `srcset`. Empty where it has no
copies.

More: https://statac.dev/templates/#files

## `fallback`

The image's JPEG or PNG copies, written as a `srcset`, for a browser
without AVIF. Empty where it has no copies.

More: https://statac.dev/templates/#files

## `sizes`

How wide the image is shown, written as the `sizes` of an `<img>`: the
call's `sizes`, or else the site's `image_sizes`.

More: https://statac.dev/templates/#files

## `full`

The URL of the image's widest copy, to link to, or with `full=true` of a
copy at the image's own width. The image itself where it has no copies.

More: https://statac.dev/templates/#files

## `width`

The image's width in pixels, once turned upright. Empty where its size
isn't known.

More: https://statac.dev/templates/#files

## `height`

The image's height in pixels, once turned upright. Empty where its size
isn't known.

More: https://statac.dev/templates/#files

## `alt`

The image's alt text, from its `.yaml` file. Empty where it has none,
and reading it then warns of the missing alt text, unless the image is
decorative.

More: https://statac.dev/templates/#files

## `caption`

The image's caption, from its `.yaml` file. Empty where it has none.

More: https://statac.dev/templates/#files

## `name`

The image's file name read as words: `photo-1.jpg` is `Photo 1`.

More: https://statac.dev/templates/#files
