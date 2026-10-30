# No thumbnail

A page shows a video hosted on YouTube or Vimeo with its `thumbnail`, but
the site's `fetched/stills/` folder has no thumbnail for that video, so
`thumbnail` is empty. The thumbnail is the image shown in place of the
video until a reader presses play.

## What you'll see {#what-youll-see}

```text
warning  No thumbnail: `statac refresh` hasn't fetched an image for this video yet
  content/post-1.md, line 1
   1 │ @youtube("https://youtu.be/abcdefghijk")
     │           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Run `statac refresh` to fetch one.
  More: https://statac.dev/e/no-thumbnail/
```

## Why Statac warns

If a page loads a video site's player as soon as it opens, the video site
learns about every reader who visits, whether or not they watch. If the
page shows a thumbnail instead, and loads the player only when the
reader presses play, the video site learns about no one until then.
`statac refresh` fetches the thumbnail from the video site once, and the
site keeps it, because `statac build` never uses the network.
That way, building the same files gives the same result every time, and
works with no connection at all.

Statac warns once for each video, however many pages show it. If a
page's content shows the video, the warning points at the call in the
Markdown. Otherwise it points at the template. If the video site replied
that it has no image for the video, `statac refresh` marks the video
`absent` in `fetched/record`. It then doesn't ask again, and the warning
says so.

## How to fix it

Run `statac refresh` in the site's folder. It fetches a thumbnail into
`fetched/stills/` for every video whose thumbnail a page uses, and lists what
it fetched. Commit `fetched/` with the rest of the site.

If the video site has no image, show one of your own. A snippet can take
your image as an argument you define, such as `poster`, and show it with
`page.image` instead of the thumbnail.

Once you have understood the warning, you can quieten it by name.

## Example

In `content/post-1.md`:

```markdown
@youtube("https://youtu.be/abcdefghijk")
```

After `statac refresh`:

```text
Fetched `fetched/stills/youtube-abcdefghijk.jpg`.
Fetched 1 thumbnail.
```

To stop Statac showing the warning, add this to `statac.yaml`:

```yaml
quiet: [no-thumbnail]
```
