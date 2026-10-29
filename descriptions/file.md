# A file in a template

Summary: Every value a template can read on a file, a video or a recording kept beside a page.

The names (variables) a template reads on the file that `page.file`
returns, and on the video or recording that `page.video` or `page.audio`
returns. `{{ page.file("notes-1.pdf") }}` on its own writes the file's
`url`.

## `url`

The file's URL, to link to.

More: https://statac.dev/videos/#files-videos-and-recordings

## `name`

The file's name, such as `notes-1.pdf`.

More: https://statac.dev/videos/#files-videos-and-recordings

## `size`

The file's size in bytes.

More: https://statac.dev/videos/#files-videos-and-recordings

## `text`

The file's contents as text, for a file of UTF-8 text. With `safe`, an
SVG's markup goes into the page itself.

More: https://statac.dev/videos/#files-videos-and-recordings

## `code`

The file's contents, ready to go inside a `<code>` element. Statac marks
each piece of the code with a class for what kind of piece it is, in the
file's language, which Statac works out from the file's name. If
`code_colours` isn't `true`, it is the file's text with no classes.
Empty for a file over 200,000 bytes or one that isn't UTF-8 text.

More: https://statac.dev/videos/#a-file-shown-as-code

## `language`

The file's language, worked out from its name and written the way a code
block names it. Empty if Statac has no colours for that language.

More: https://statac.dev/videos/#a-file-shown-as-code

## `src`

The URL of the video or recording, for the `src` of a `<source>`.

More: https://statac.dev/videos/#files-videos-and-recordings

## `type`

The media type of the video or recording, for the `type` of a
`<source>`: `video/mp4` for `video-1.mp4`.

More: https://statac.dev/videos/#files-videos-and-recordings
