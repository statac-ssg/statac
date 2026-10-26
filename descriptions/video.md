# A video in a template

What a template reads on the video that `page.youtube` or `page.vimeo`
returns.

## `id`

The video's id on YouTube or Vimeo.

More: https://statac.dev/templates/#files

## `start`

The second the video starts at, taken from the `t` or `start` in its
URL. If the URL has neither, `start` has no value.

More: https://statac.dev/templates/#files

## `embed`

The URL of the player, to put in an `<iframe>`. If the video has a
`start`, the player starts there.

More: https://statac.dev/templates/#files

## `thumbnail`

The video's thumbnail, with the same values as an image from
`page.image`. It has no value until `statac refresh` fetches it.

More: https://statac.dev/templates/#files
