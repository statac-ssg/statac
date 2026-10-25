# A video in a template

What a template reads on a video `page.youtube` or `page.vimeo` gives.

## `id`

The video's id on its host.

More: https://statac.dev/templates/#files

## `start`

Where the video starts, in seconds, from the `t` or `start` in its URL.
A URL without one has none.

More: https://statac.dev/templates/#files

## `embed`

The URL of the host's player, for an `<iframe>`, starting at `start`
where there is one.

More: https://statac.dev/templates/#files

## `thumbnail`

The video's thumbnail, as `page.image` gives an image, once `statac
refresh` has fetched it. Until then it has none.

More: https://statac.dev/templates/#files
