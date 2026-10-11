# No still

A page shows a video hosted on YouTube or Vimeo, and uses the video's
`still`: the picture shown in place of the video until a reader presses
play. The site's `fetched/stills/` folder has no still for this video, so
`still` is empty.

## What you'll see {#what-youll-see}

```text
warning  No still: no picture has been fetched for this video yet
  Run `statac refresh` to fetch one.
  More: https://statac.dev/e/no-still/
```

## Why Statac warns

If a page loads a video site's player as soon as it opens, the video site
learns about every reader who visits, whether or not they watch. If the
page shows a still instead, and loads the player only when the reader
presses play, the video site learns about no one until then.
`statac refresh` fetches the still from the video site once, and the
still is kept in the site, because `statac build` never uses the network.
That way, building the same files gives the same result every time, and
works with no connection at all.

Statac warns once for each video, however many pages show it. If your
writing shows the video, the warning points at the call in the writing.
Otherwise it points at the template. If the video site replied that it has
no picture for the video, `fetched/record` marks the video `absent`.
`statac refresh` then doesn't ask again, and the warning says so.

## How to fix it

Run `statac refresh` in the site's folder. It fetches a still into
`fetched/stills/` for every video whose still a page uses, and lists what
it fetched. Commit `fetched/` with the rest of the site.

If the video site has no picture, show one of your own. A snippet can take
your picture as an argument you define, such as `poster`, and show it with
`page.photo` instead of the still.

Once you have understood the warning, you can quieten it by name.

## Example

In `content/post-1.md`:

```markdown
@youtube("https://youtu.be/abcdefghijk")
```

After `statac refresh`:

```text
Fetched `fetched/stills/youtube-abcdefghijk.jpg`.
Fetched 1 still.
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [no-still]
```
