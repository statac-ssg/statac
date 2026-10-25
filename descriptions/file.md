# A file in a template

What a template reads on a file `page.file` gives, and on a video or a
recording `page.video` or `page.audio` gives. Shown as it is, a file
`page.file` gives is its `url`.

## `url`

The file's URL, to link to.

More: https://statac.dev/templates/#files

## `name`

The file's name, as `notes-1.pdf`.

More: https://statac.dev/templates/#files

## `size`

The file's size in bytes.

More: https://statac.dev/templates/#files

## `text`

The file's contents as text, for a file of UTF-8 text. With `safe`, an
SVG's markup goes into the page itself.

More: https://statac.dev/templates/#files

## `code`

The file's contents as code to go inside a `<code>` element, each piece
marked with a class for its kind in the language its name gives. Where
`code_colours` isn't `true`, the text as written. Empty for a file over
200,000 bytes or one that isn't UTF-8 text.

More: https://statac.dev/templates/#files

## `language`

The language the file's name gives, as a code block names it. Empty
where Statac has no colours for it.

More: https://statac.dev/templates/#files

## `src`

The URL of the video or recording, for the `src` of a `<source>`.

More: https://statac.dev/templates/#files

## `type`

The media type of the video or recording, for the `type` of a
`<source>`: `video/mp4` for `video-1.mp4`.

More: https://statac.dev/templates/#files
