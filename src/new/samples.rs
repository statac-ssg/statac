//! The sample pages of a new site: two posts, three notes and six
//! photographs, each dated today or some days before, so that a new
//! site's lists have an order, and a page with no date. Each is written
//! as it is here with its day in place of `{day}`.

use crate::date::Date;

/// A sample page: its path in a site, how many days before today it is
/// dated, and its text.
const PAGES: &[(&str, i32, &str)] = &[
    (
        "content/about.md",
        0,
        r#"---
title: About
menu: true
---
This is a page that isn't a post, the file `content/about.md`. It has no date and is listed with no posts. Write here what the site is about and who writes it, in place of these words.

The `menu: true` line in the file puts the page in the site's menu, after the folders. To make another page, add a file beside this one in `content/`, such as `contact.md`, opening with its title:

```yaml
---
title: Contact
menu: true
---
```

Then write the page below that, in Markdown. It is published at `/contact/`. Without the `menu: true` line it is published all the same, and is reached by the links you write to it.
"#,
    ),
    (
        "content/posts/first-post/index.md",
        0,
        r#"---
title: First post
date: {day}
cover: photo-1.jpg
tags: [Examples]
---
This is the site's first post, the file `content/posts/first-post/index.md`. Its picture is `photo-1.jpg`, kept beside it and named by its `cover:` line, and what a reader who can't see the picture is told in its place is in `photo-1.jpg.yaml`. Change them or remove them as you like.

<!--more-->

In the file, a `<!--more-->` line stands above this paragraph and marks where the post's opening ends: a list of posts shows the post as far as there, with a link to the rest. A post without the line is shown as far as the end of its first paragraph.

To write the next post, add a file beside this folder in `content/posts/`, such as `second-post.md`, opening with its title and its date:

```yaml
---
title: Second post
date: =today
---
```

Then write the post below that, in Markdown. `statac serve` puts the day in place of `=today` when you save. A post with a picture is a folder like this one, its writing in `index.md` and the picture beside it. A post in `content/posts/` is listed on the home page, the newest first.
"#,
    ),
    (
        "content/posts/what-a-post-can-hold/index.md",
        1,
        r#"---
title: What a post can hold
date: {day}
summary: Every kind of thing a post can show, each beside the way it is written, so the site's look can be judged on one page.
tags: [Examples]
contents: true
---
This post shows each thing a page's writing can hold, with the way it is written in the file, `content/posts/what-a-post-can-hold/index.md`. Change `assets/site.css` and look here to see what the change did. Remove the post when you no longer want it.

A paragraph is a line or more of writing with an empty line after it. In it, `*emphasis*` is *emphasis*, `**strong**` is **strong**, a pair of backticks holds `code`, and `[words](https://example.org/)` is [a link](https://example.org/). A link to a page of this site is written with its address in `@url(...)`, as [the first post](@url(/posts/first-post/)), so it still leads there if the site moves under a path. HTML is taken as written: <kbd>Ctrl</kbd> + <kbd>S</kbd>, <mark>marked words</mark>, <abbr title="HyperText Markup Language">HTML</abbr>.

## Headings

A line starting `##` is a heading. The page's title is its only first heading, so headings in the writing start at the second level. With `contents: true` in the front matter, as here, they are listed at the top of the post.

### A third-level heading

Three `#` signs make a heading beneath that one.

#### A fourth-level heading

Four make one beneath that.

## Lists

Lines starting `-` are a list:

- one thing
- another
  - a list inside it, set in by two spaces
  - and its second line
- a third

Lines starting with a number are a list in order:

1. First this.
2. Then this.
3. And this last.

Lines starting `- [ ]` are boxes to tick:

- [x] A box that is ticked
- [ ] A box that is not

A term on one line and `: ` before its meaning on the next are a list of terms:

Title
: What the page is called, in its front matter.

Summary
: A line about the page, shown under its title, in lists and in a link's preview.

## Quotations

A line starting `>` is a quotation:

> Words quoted from somewhere else, set apart from the writing around them.

To say who said it and where, wrap the words in `@quote`:

@quote("Who said it", cite="Where it was said")
Words quoted with their source beneath them.
@end

## Code

Three backticks on a line of their own open and close a block of code. A language named after the opening backticks colours it:

```yaml
title: What a post can hold
date: {day}
tags: [Examples]
```

A file's name there colours the code by the name and is shown above it:

```css site.css
:root {
  --column: 40rem;
  --wide: 58rem;
}
```

`@file("example.css")` shows a file kept beside the post, with a link to it:

@file("example.css")

## Pictures

A picture is kept beside the post and named in Markdown, as `![](photo-3.jpg)`. What a reader who can't see it is told is in the file `photo-3.jpg.yaml`.

![](photo-3.jpg)

`@figure` gives a picture a caption:

@figure("photo-4.jpg", caption="A caption, in smaller and quieter words than the writing.")

With `wide=true` the picture is wider than the writing:

@figure("photo-2.jpg", wide=true, caption="A wide figure.")

`@gallery` sets several pictures side by side, each a link to its largest copy. With no pictures named, it shows every picture beside the post.

@gallery("photo-3.jpg", "photo-4.jpg", "photo-5.jpg")

## Videos and recordings

`@video` shows a video kept beside the post, and `@audio` a recording, each with a caption where it is given one:

@video("video-1.mp4", caption="A video of two seconds.")

@audio("recording-1.m4a", caption="A recording of two seconds.")

A video kept on YouTube or Vimeo is written `@youtube("abcdefghijk")` or `@vimeo("123456789")`, with the video's own letters or figures from its address. It is shown as a still with a button to play it, and nothing is asked of the other site until a reader presses it.

## Tables

Rows of cells parted by `|`, with a row of dashes under the first, are a table. A colon at the end of the dashes sets a column to the right, for figures. A table too wide for the page scrolls in its own box.

| Size | Name | Width |
|---|---|---:|
| Small | `--small` | 15 |
| Text | `--text` | 17 |
| Large | `--large` | 20 |

## Notes and callouts

A quotation that opens `[!NOTE]` is a callout. The other kinds are `TIP`, `IMPORTANT`, `WARNING` and `CAUTION`.

> [!NOTE]
> A callout holds something beside the main point.

> [!WARNING]
> Each kind is named in words, from `data/words.yaml`.

`@note()` and `@end` set a note apart without a name:

@note()
A note, in a box of its own.
@end

HTML's own `<details>` hides what only some readers want, with no script:

<details>
<summary>What is shown until it is opened</summary>
<p>What is shown once it is.</p>
</details>

## Footnotes and maths

`[^1]` in the writing points at a footnote[^1], written anywhere in the file as `[^1]: ` and its words. Footnotes are gathered at the end.

Maths between dollar signs is drawn by the browser, with no script: $a^2 + b^2 = c^2$ in a line, or set apart between two pairs:

$$
\sum_{i=1}^{n} i = \frac{n(n+1)}{2}
$$

---

Three dashes on a line of their own are a break, as above. That is everything the stylesheet has a look for. If a post needs something that isn't here, give it a look in the part of `assets/site.css` called "The writing".

[^1]: This is the footnote. The arrow after it leads back to where it was pointed at.
"#,
    ),
    (
        "content/notes/note-1.md",
        0,
        r#"---
date: {day}T09:12
tags: [Examples]
---
This is a note, the file `content/notes/note-1.md`: a few lines with a date and no title. Its front matter holds only the date.
"#,
    ),
    (
        "content/notes/note-2.md",
        1,
        r#"---
date: {day}T16:20
---
A note can hold anything a post can:

- a list
- [a link](@url(/))
- a picture, where the note is a folder with the picture beside it
"#,
    ),
    (
        "content/notes/note-3.md",
        2,
        r#"---
title: A note with a title
date: {day}T07:40
---
A note may have a title, and then the title is shown with its date. To do without notes, delete the folder `content/notes/`.
"#,
    ),
    (
        "content/photos/photo-1/index.md",
        3,
        r#"---
title: Photo 1
date: {day}
cover: photo-1.jpg
---
This is a photograph's page, the file `content/photos/photo-1/index.md`, with the picture beside it and named by its `cover:` line. Write about the picture here. To do without photographs, delete the folder `content/photos/`.
"#,
    ),
    (
        "content/photos/photo-2/index.md",
        5,
        r#"---
title: Photo 2
date: {day}
cover: photo-2.jpg
---
A gradient from plum to peach.
"#,
    ),
    (
        "content/photos/photo-3/index.md",
        7,
        r#"---
title: Photo 3
date: {day}
cover: photo-3.jpg
---
A gradient from violet to pale blue.
"#,
    ),
    (
        "content/photos/photo-4/index.md",
        18,
        r#"---
title: Photo 4
date: {day}
cover: photo-4.jpg
---
A gradient from blue to gold.
"#,
    ),
    (
        "content/photos/photo-5/index.md",
        25,
        r#"---
title: Photo 5
date: {day}
cover: photo-5.jpg
---
A gradient from teal to sand.
"#,
    ),
    (
        "content/photos/photo-6/index.md",
        36,
        r#"---
title: Photo 6
date: {day}
cover: photo-6.jpg
---
A gradient from rust to rose.
"#,
    ),
];

/// Every sample page, by its path in the site, dated from `today`.
pub(super) fn pages(today: Date) -> Vec<(String, String)> {
    PAGES
        .iter()
        .map(|(path, days, text)| {
            let day = days_before(today, *days).iso_day();
            ((*path).to_owned(), text.replace("{day}", &day))
        })
        .collect()
}

/// The day `days` before `today`; `today` itself where the calendar has
/// no such day.
fn days_before(today: Date, days: i32) -> Date {
    let before = i16::try_from(today.year)
        .ok()
        .zip(i8::try_from(today.month).ok())
        .zip(i8::try_from(today.day).ok())
        .and_then(|((year, month), day)| jiff::civil::Date::new(year, month, day).ok())
        .and_then(|day| day.checked_sub(jiff::Span::new().days(days)).ok());
    match before {
        Some(day) => Date::day_only(
            i32::from(day.year()),
            day.month().unsigned_abs(),
            day.day().unsigned_abs(),
        ),
        None => today,
    }
}
