# An image in a template

What a template reads on the image that `page.image` returns, and on
each image from `page.images`, a video's `thumbnail`, a mention's
`avatar`, and `image` in `image.html`. `{{ image }}` on its own writes
the image's `src`. A template reads a custom field in the image's
`.yaml` file the same way, such as `image.credit`.

## `src`

The image's URL, for the `src` of an `<img>`: its widest resized copy no
more than 1200 pixels wide, or the image itself if it has no copies.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `avif`

The image's AVIF copies, written as a `srcset`. Empty if it has no
copies.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `fallback`

The image's JPEG or PNG copies, written as a `srcset`, for a browser
without AVIF. Empty if it has no copies.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `sizes`

How wide the browser shows the image, written as the `sizes` of an
`<img>`. It is the `sizes` the call gives, or if the call gives none,
the site's `image_sizes`.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `full`

The URL of the image's widest copy, to link to. With `full=true`, it is
the URL of a copy at the image's own width. If the image has no copies,
it is the URL of the image itself.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `width`

The image's width in pixels, after Statac turns it the right way up.
Empty if Statac doesn't know its size.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `height`

The image's height in pixels, after Statac turns it the right way up.
Empty if Statac doesn't know its size.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets

## `alt`

The image's alt text, from its `.yaml` file. If the image has none,
`alt` is empty, and Statac warns that the alt text is missing when a
template reads it, unless the image is decorative.

More: https://statac.dev/pictures/#describing-a-picture

## `caption`

The image's caption, from its `.yaml` file. Empty if it has none.

More: https://statac.dev/pictures/#describing-a-picture

## `name`

The image's file name turned into words: `photo-1.jpg` becomes
`Photo 1`.

More: https://statac.dev/pictures/#pictures-in-templates-and-snippets
