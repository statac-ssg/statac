# No still

A page shows a video kept on YouTube or Vimeo, and reads the video's
`still`, the picture shown in its place until a reader presses play. No
still for the video is in the site's `fetched/stills/` folder, so the
page has none: `still` is empty.

## What you'll see {#what-youll-see}

```text
warning  No still: no picture has been fetched for this video yet
  Run `statac refresh` to fetch one.
  More: https://statac.dev/e/no-still/
```

## Why Statac warns

A page that loads a video site's player as it opens tells that site of
every reader who visits, whether or not they watch. Showing a still
instead, and loading the player only when the reader presses play, tells
it of no one until then. The still is fetched from the video site once,
by `statac refresh`, and kept in the site, since `statac build` never
uses the network: a build of the same files is the same every time, and
works with no connection at all.

Statac says so once for each video, however many pages show it: at the
call in the writing where the writing shows the video, and otherwise in
the template. Where the video site answered that it has no picture for
the video, `fetched/record` says `absent` against it, `statac refresh`
doesn't ask again, and the warning says so.

## How to fix it

Run `statac refresh` in the site's folder. It fetches the still of every
video a page reads one for, into `fetched/stills/`, and says what it
fetched. Commit `fetched/` with the rest of the site.

Where the video site has no picture, show one of your own: a snippet can
take your picture as an argument of its own, such as `poster`, and show
it with `page.photo` in place of the still.

Once you have understood it, the warning can be quietened by name.

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
