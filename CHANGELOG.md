# Changes

What changed in each release of Statac, the newest first.

## 0.7.3 - 2026-10-30

### Fixed

- `cargo install --locked statac`, like any build of Statac from its
  source, needs a C++ compiler as well as a C compiler. If the C++
  compiler is missing, the build now stops before CMake runs, with a
  message that names the compiler it looked for and says to install
  one, such as `g++`, or to set `CXX` to the compiler to use. With
  Statac 0.7.2, the build stopped deep in CMake's output on a machine
  without one, such as Debian, Ubuntu or Fedora with only the `gcc`
  package.
- `IMPORT.md` names every home page the import wrote with no content,
  such as `content/index.md` and, on a site in several languages,
  `content/fr/index.md`, and says it has no content. Statac 0.7.2 named
  only `content/index.md`, so the report didn't tell you that each
  other language's home page needed writing too. It also said the file
  was empty, but where the import writes front matter there, such as
  `posts_per_page: 0` for a paginated home page or `feed:` for the home
  page's feed, it isn't.
- When a page's `og_image` names an image that Statac can't read
  because of the form it is saved in, Statac suggests exporting the
  image again as an ordinary JPEG, PNG or WebP. Statac 0.7.2 suggested
  `optimise: false` in the image's sidecar file, which doesn't help
  there: Statac needs the image's size for the social card. Where the
  content shows the same image, the suggestion is still `optimise:
  false`, which works there.
- The debug panel shows a problem's explanation with its links as
  links, opening statac.dev in a new tab, its bold text as bold, and
  its lists as lists. Statac 0.7.2 showed the Markdown as written, so
  a link read as `[text](url)` and a bold lead-in as `**text**`.
- The Examples of five explanations now give the problem they explain,
  so a site that follows one meets the message under "What you'll
  see": `same-id` shows two posts with the same `id:`,
  `page-in-snippet` a snippet that also reads the post it is handed,
  `unused-card` a whole card file in a site whose layout reads
  `page.card`, `hook-stopped` a `before_build` hook naming a file that
  isn't there, and `cant-write` a read-only `_site`. Before,
  `page-in-snippet`'s and `unused-card`'s Examples stopped with another
  error, `same-id`'s and `cant-write`'s gave no problem, and
  `hook-stopped`'s showed a message that wasn't what Statac prints.
  `cant-write`'s explanation also says that Statac reports it for a
  file in `content/` whose name can't be part of a URL, such as one
  with `#` or `%` in it, and no longer suggests moving a folder that is
  in the way, which a build replaces.
- The explanation of `unreadable-image` gives each of the five reasons
  the message can give, with a fix for each: a file cut short or
  damaged, one saved in a form Statac doesn't read, an image too large
  to open, a resized copy Statac can't make, and a file in `fetched/`
  that isn't a JPEG, a PNG or a still WebP. It also says that only
  `build` finds an image too large to open or a copy it can't make, and
  how to publish an image in `static/` unchanged. Statac 0.7.2's
  explanation described only the first two reasons.

### Changed

- Each explanation, on its page at `https://statac.dev/e/<name>/` and
  in `statac explain <name>`, shows the message under "What you'll
  see" whole, in the form Statac prints it: the headline, then the file
  and line, the line itself with carets under the text, the suggestion
  and the `More:` line. A message about a page's file follows the
  page's own Example. A message in no file, such as a full disk or a
  mistake on the command line, has no location lines, as in the
  terminal. The pages no longer say that the terminal adds the file
  and line.
- The list of problems at https://statac.dev/e/ follows each kind's
  name with the first sentence of its explanation, so you can find a
  kind by what happened, not only by its name. Eleven explanations
  open with a new first sentence for it, which `statac explain`, the
  debug panel and an editor's hover show too: `alias-limit`,
  `cant-tell-what-made`, `field-written-two-ways`,
  `footnote-never-shown`, `loop-over-every-page`, `missing-anchor`,
  `missing-translation`, `no-thumbnail`, `not-stamped`,
  `other-spelling` and `unusable-date-format`.
- Where a kind of problem has several causes, its explanation, on
  statac.dev and in `statac explain`, starts the fix for each cause
  with that cause in bold, so you can find the fix for yours without
  reading the others: `card-not-drawn`, `feed-not-written`,
  `missing-template`, `no-value`, `redirect-to-itself`, `same-url`,
  `unknown-word`, `unmatched-tag` and `unused-card`.
  `feed-not-written` links to https://statac.dev/feeds/#the-template
  for the order in which Statac looks for a feed's template and the
  names the template is handed, in place of setting them out, and
  `no-value` links to https://statac.dev/templates/#no-value for what
  each filter does with a page that lacks a field, and to
  https://statac.dev/upgrading/#other-stops for tag pages on a site
  made before 0.7.0.
- The explanation of `colour-converted` gives as its Example the
  sidecar file `content/photo-1.jpg.yaml` with `optimise: false`, with
  which Statac publishes the image's own file, with its Display P3
  profile, in place of resized copies, and shows no note. The Example
  used to repeat the `quiet:` line. The explanation of `url-gone` says
  why `published.txt` exists: if a URL in it stops being published,
  Statac tells you, so links other people have to your site don't
  quietly break.
- Each of the docs' Reference pages that describes the names a
  template can read writes "names (variables)" where it first uses the
  word, so the pages use the same term as the docs' guides. The page of
  names a template is handed is titled "Names a template is handed
  (variables)", so a search for variables finds it. The editor's hover
  is unchanged.

## 0.7.2 - 2026-10-28

### New

- You can install Statac from crates.io with `cargo install --locked
  statac`. Cargo builds Statac from its source, so this needs Rust 1.99
  or later, a C compiler, CMake and Perl, and NASM on a machine with an
  Intel or AMD processor. If CMake, Perl or NASM is missing, the build
  stops with a message that names it and says where to get it.

### Fixed

- Statac writes the text of its name badge, and of the `warning` and
  `note` badges, in a fixed black, so the text stays black in a
  terminal that brightens bold text, as iTerm2 does by default. Statac
  0.7.1 used the terminal's own black, which such a terminal turns
  grey.

## 0.7.1 - 2026-10-28

### Fixed

- Statac stops the build at a template name that 0.7.0 renamed, such
  as `page.photo`, `mentions.faces` or `mention.face`, wherever a
  template reads it, and offers the new name: inside `{% if %}`, in a
  loop's `if`, in `selectattr` and `rejectattr`, and inside a branch
  whose test read it. Statac 0.7.0 told it only outside a test, so a
  template that tested an old name showed nothing there, and nothing
  said so. This means a template that 0.7.0 built without a word can
  now stop the build, with `other-spelling` naming the new name to
  write. Statac still reads `photo`, `photos` or `words` from a page
  that sets it, or where `statac.yaml` declares it, since the field is
  then the page's own.
- Where a site has no `published.txt` and its `_site` holds an earlier
  Statac build, Statac reads the URLs in `_site` in the file's place,
  so `statac check` and the first build warn of each page and feed the
  site no longer publishes (`url-gone`), such as a folder's feed or a
  paginated page, which 0.7.0 makes only where the site asks. Until the
  first build writes the file, each warning says where its URL came
  from: "Published by an earlier build, in `_site`". The build starts
  `published.txt` with those URLs and says so. A build with `--drafts`
  or `--url` that replaces `_site` starts the file with them too, so
  they aren't lost with the folder. Statac 0.7.0 started the file
  afresh, so a site upgraded from 0.6 was told nothing of what it no
  longer published.
- Where a template reads a tag's or a series' `url` and the site
  doesn't publish tag or series pages, `no-value` offers the setting
  to add, `tags.pages: true` or `series.pages: true`, as `statac
  check`'s "No list page" note does. It offered a `default` filter,
  which would hide the links.
- Statac's messages, the descriptions an editor shows on hover, the
  comment at the top of the language file `statac new` writes, and
  `statac import`'s notes on a site in several languages link to
  https://statac.dev/dates/ and https://statac.dev/languages/, where
  the docs moved the sections of /dates-and-languages/. The old links
  still lead to the right section, so a language file written before
  needs no change.
- When you give `statac new` a language Statac has no translations
  for, its note links the section on writing that language's file,
  https://statac.dev/languages/#language-files, in place of the section
  on languages as a whole.
- The descriptions an editor shows on hover and the debug panel shows
  in its Names tab say "list page" where they said "the list's page"
  or "the list's own page", and `page.previous` and `page.next` on a
  list page say "the previous page of the list" and "the next page of
  the list". `feed-not-written`'s explanation and suggestion say "list
  page" too: "A feed links to its list page; add an `index.md` to give
  the list one, or take the `feed:` line out."

## 0.7.0 - 2026-10-27

A breaking release. Statac writes no templates into a site, makes only
the pages and feeds a site asks for, and takes the words people search
for. Before upgrading a site made with 0.6, read
https://statac.dev/upgrading/, which says what to change and what
changes without a word.

### Breaking

- Statac writes no templates into a site. `statac new` writes
  `statac.yaml` and an empty `content/`, and `statac import` brings
  across the content and settings alone. Statac refuses `statac new`'s
  `--update`, `--missing`, `--cards` and `--no-starter`. The docs'
  tutorial has templates to copy for the parts a site may want.
- Statac makes a list page only where the site asks for one: the home
  page needs `content/index.md` or `templates/home.html`, a folder's
  page its `index.md`, tag and series pages `tags.pages: true` or
  `series.pages: true`, and later pages `posts_per_page`. `statac check`
  says once what an existing site no longer gets, and the line to add.
- A list has a feed only where a `feed:` line asks for one: `feed: true`
  in `statac.yaml` for the site's feed, and in a folder's `index.md`
  for the folder's. Both had one by default. Statac warns of a list that
  asks for a feed and has nothing dated, notes a feed template no list
  asks for, and publishes no feed for a list whose page isn't published,
  since a feed links to its list's page; `feed-not-written` says so.
- Settings, template names and problem names take the words people
  search for. Renamed settings: `later_pages` is
  `pagination.permalink`, `left_behind` is `exclude`, `also_at` is
  `other_hosts`, `not_ours` is `served_elsewhere`, and
  `after_build_makes` is `after_build_outputs`. Renamed in templates:
  `page.photo()` and `page.photos()` are `page.image()` and
  `page.images()`, `page.words` is `page.word_count`, a video's `still`
  is `thumbnail`, `page.webmentions.said` and `.faces` are `.comments`
  and `.reactions`, and a webmention's `face` and `words` are `avatar`
  and `content`. Statac hands Markdown snippets `url`, `external` and
  `image`. Problem names change too, such as `undescribed-picture` to
  `missing-alt-text`, `waiting-for-its-day` to `scheduled-post` and
  `missing-words` to `missing-translation`. An old name stops the
  build, and Statac offers the new one.
- Statac fills `image:` and `images:` on every page, so it sets aside a
  page's own field of either name: rename the field.
- Messages, explanations, descriptions and `statac --help` say URL,
  slug, pagination, scheduled post, image, content, translation and
  social card.
- Statac colours code only where `code_colours: true` asks, and writes
  no `code.css`. It refuses `code_style`; the docs give each theme's
  rules to copy into a stylesheet of the site's own.
- Statac writes dates as each language writes them in the Unicode
  CLDR. English follows its region: `en` and `en-US` write "December
  31, 2025", and `en-GB`, `en-AU` and the other regions "31 December
  2025". A site with `language: en` keeps day-first dates with
  `date_format: 31 December 2025`. A region of any language Statac
  carries, such as `es-MX`, `fr-CA` or `de-AT`, writes its dates as the
  CLDR writes that region, and a month and year, or a year alone, comes
  in the language's own order.
- `statac refuse` is gone. A site lists the webmentions it refuses in
  `statac.yaml` as `refuse_webmentions_from:`, one full URL a line; each
  refuses every mention sent from a page at or under it, or by a sender
  whose URL is. A build leaves them out at once, and `statac refresh`
  deletes their files for good. A site with `fetched/refused` stops,
  and Statac tells it the lines to add.
- `--site=` is gone: name the site's folder after the command, as
  `statac build docs`. Statac refuses `--site=` with that suggestion.
- `statac new` asks for the site's title, the author's name, the site's
  URL, its language and its time zone, and writes only what you answer.
  It writes no date format and no `feed_key`; `statac check` asks for a
  feed key once the site has a feed.
- `pages.order` with a list of names stops the build with
  `wrong-place`. Handed down, a list of names did nothing, and nothing
  said so; write it as `order:` in the `index.md` of the folder it
  orders.

### New

- `statac lsp`, a language server, gives an editor what `statac check`
  knows, as you type. https://statac.dev/editors/ says how to start it
  in Sublime Text, VS Code, Zed and other editors.
  - It shows every problem where it is written, with its suggestion and
    a link to its explanation. A third of a second after you stop
    typing, it checks the unsaved text, building again only what the
    change touched, and it checks again when a file of the site
    changes outside the editor.
  - It completes `statac.yaml` and front matter: settings, fields, the
    fields a site declares, and their values, among them the site's
    tags with how many pages carry each, its templates and its series.
    In content it completes snippet calls with what each takes, the
    files beside the page, the site's URLs in links and `@url(...)`,
    and a page's headings after `#`. In templates and snippets it
    completes the names and fields readable where the cursor is,
    filters, tests, statements, the blocks of the template extended,
    and the templates, URLs and files a template names.
  - Hover says what each setting, front matter field, template name,
    filter and function does, with a link to the docs; what a link
    leads to; what a snippet call names; which file beside the page a
    value names; and how many pages carry a tag or series.
  - Go to definition goes from a link or `@url(...)` to the page or
    heading, from a snippet call to the snippet, from `layout:` or
    `extends` to the template, from an image or file a page names to
    that file, and from a custom field to its declaration in
    `statac.yaml`. Find references lists every link to a page or a
    heading on it, from content and templates, counting links to its
    old URLs. The outline lists a page's headings, and the editor's
    symbol search finds pages by title and snippets and templates by
    name.
  - It offers a fix where a problem's suggestion names one exact
    answer: a misspelt word, name, label, block, argument or file,
    another tool's spelling, a URL or anchor, a template, a value
    needing quotes or brackets, or either reading of an unclear date.
  - When you rename a page's file or folder in the editor, it rewrites
    every link to the page in the form it was written in, and adds the
    old URL to the page's `redirect_from:`.
  - It works on drafts and scheduled posts too: an open one is checked
    as if published, and find references and rename count the links
    drafts write.
- `published.txt` keeps every URL the site publishes. Each build and
  `statac serve` add to it, and `statac check` warns when a URL stops
  answering, with the `redirect_from` line that sends readers on; a
  folder gone whole is one warning. Commit it with the site, and take
  out the line of a page that's gone for good, which the editor offers
  to do. The first build that publishes a page starts it.
- `statac check --json` prints every problem as one JSON document, for
  an editor, a CI job or another program to read: each problem's
  severity, name, message, suggestion and explanation, with its file
  and where in it the problem starts and ends, as a line and column
  and as a byte offset. Quietened problems are there too, marked.
- `statac build --timings` shows where a build spent its time: each
  stage, the work of building the pages across threads, and the five
  slowest templates, pieces of content, pages and images.
- With no home page, `statac serve` answers the top of the site with a
  page listing every page the build made, each linked, with drafts and
  scheduled posts marked. `statac build` doesn't publish it.
- Every setting, front matter field, template name, filter, function
  and test has a description, which the editor shows on hover and in
  completion, the debug panel shows in its Names tab, and the docs'
  Reference pages give. Each links to the docs section that teaches
  the name.
- Statac checks the fields a template reads on an image, a file, a
  video or recording, a social card, a heading in `page.toc` and a
  search box when it reads the template, and offers the nearest field
  for a misspelt one.
- `statac check` notes a link or a button with nothing a screen reader
  can read out, an `<iframe>` with no `title`, a form field with no
  label, and a page with no `<title>`, each at the line that wrote it.
  A link or a button in content that shows only an image with no alt
  text is told once, as the image's missing alt text, which should
  then say where the link leads or what the button does.
- A task list's checkbox is labelled with its item's words, so a
  screen reader names it.
- `feed:` takes a list of names, each feed built from the template
  named for its file or by its extension, the first one advertised:
  `feed: [feed.xml, rss.xml]` publishes Atom and RSS side by side. Feed
  templates are handed `feed`, their own URL. Statac warns of a feed
  whose name says one format, such as `rss.xml`, when the build makes
  another (`feed-format-mismatch`).
- A template names a date's look: `date("long")`, `date("short")`,
  `date("day")`, `date("stamp")`.
- `statac check` notes a file in `languages/` that no language of the
  site reads, such as `languages/en-US.yaml` on a site in `en-GB`
  (`unused-language-file`), and offers the site's languages.
- The install script, `curl -fsSL https://get.statac.dev | sh`,
  downloads each release from get.statac.dev. Statac's code, issues,
  Homebrew tap and Scoop bucket are at git.statac.dev. Report a
  security problem to security@statac.dev.

### Changed

- Every message, explanation and description says who does what:
  "Statac reads dates without an offset as UTC" where it said "dates
  without an offset are read as UTC", and "Told the browser to reload."
  where it said "Browser told to reload.". The explanations and
  descriptions say what several names do that they left out, and
  several that misdescribed tag pages, missing translations, social
  cards and check-only notes are corrected.
- Search ranks a part first by where a word stands, then by how often:
  a word in a page's title or a part's heading counts for more than the
  same word written several times in a short part, and a page whose
  title is only the word comes before one whose longer title holds it.
  Serve's Search tab shows what each place adds to a result's score.
- `exclude` takes a path from `content/`, such as `blog/one.md`, as
  well as a name.
- Two files published at one URL are told at the `redirect_from` or
  `permalink` that puts one there, with advice to change it, not to
  rename a file. A page whose `redirect_from` names its own URL is told
  at that line. A draft whose URL another page's `redirect_from` takes
  is told to take it out of `redirect_from` there, or to give the draft
  a `permalink`.
- A broken link is told where its URL was written: in a front matter
  field or data file a template prints, in a value shown through
  `markdown` or `safe`, inside a macro or a snippet the content calls,
  and at each URL a `srcset` lists. A link written in content and
  again in a template is told once at each place, not a third time at
  `{{ page.content }}`.
- `statac check`'s list of JavaScript the site carries counts an embed
  by the site it comes from, so a site's videos from one site make one
  line, not one line each.
- When a build stops, `statac serve` says it keeps serving the
  previous build, and the debug panel's error card says when the page
  it shows was built.
- An unknown command is told alone, without an error for each argument
  after it, and `statac <unknown> --help` stops with the unknown
  command, as the command alone does.
- The language file `statac new` writes for a language with no
  translations opens with a full link to its docs.
- A region takes the Unicode CLDR's quotation marks where they differ
  from its language's: Spanish in the United States and French in
  Canada and Switzerland.
- A language code naming its language's usual script, such as
  `en-Latn-GB`, takes the translations, dates and language file of the
  code without it, `en-GB`; a region takes the site's file for the
  broader form of its language, `en-GB` taking `languages/en-001.yaml`
  before `languages/en.yaml`. A language written in another script,
  such as `en-Shaw` or `ja-Latn`, takes none of the language's
  translations or files, and is `unknown-language` until the site has
  a file for it.
- On Windows, Statac passes over a kept copy of an image or a social
  card, the images' index and the feeds' record of posts if the file
  has a second name somewhere, as it does on macOS and Linux, and
  works out what it held again. A kept copy given a second name while
  a build runs is never copied into the built site.

### Fixed

- A line in a site's language file that the file can't set, such as
  `dates`, is told as in the wrong place, and any other line the file
  doesn't know as an unknown line. Both were told as unknown settings.
- A setting written for one language, such as `fr.title`, on a site in
  one language is told, with the setting itself offered.
- A site setting written nested in a page's front matter is told as in
  the wrong place, named whole.
- For a list page Statac makes, such as a tag's page, a missing
  template's suggestion offers a page of your own at its URL. It
  offered `layout:`, which can't be written there.
- A video shown whole, or handed whole to a snippet, gives its id and
  needs no `start`. Statac asked for one.
- `statac refresh` fetches the thumbnails a site lacks when the only
  problems stopping the build are values those thumbnails give, then
  reads the site again. It stopped without fetching them.
- Statac checks a feed's template as reading a site with a `url` and a
  `title`, since it builds a feed only for such a site. It told the
  site it lacked them.
- A build keeps the time of each file it doesn't change, and gives a
  file changed back to what it was a newer time.
- `statac serve` builds again only the pages an edit reaches after a
  social card's background or font, or a file a page shows, was saved
  just before a build. Where the disk's file times move in steps, or
  the file was dated in the future, the next build made every page
  again, whatever the edit.
- Statac refuses an English month or weekday in another language's
  example date, and offers the language's own word.
- A `date_format` message offers examples as the site's language
  writes them: `12/31/2025` and `Wednesday, December 31, 2025` for a
  site in `en-US`.
- In a `date_format` or a template's example date, a slip in a
  weekday's or month's name stops as the name written correctly does,
  and Statac offers the reference date's: `Tusday 31 December 2025`
  offers `Wednesday 31 December 2025`. In Chinese, Japanese and Korean
  a slip in a short word, as `水曜目` for `水曜日`, is refused with the
  word offered, and two words run together, as `水曜日午後3:04`, are
  refused with the format written apart offered. Statac copied each
  into every date as text.
- Statac reads a `date_format` written as Japanese writes its full
  date, `2025年12月31日水曜日`, and a weekday written straight after the
  day's mark with a site's own Chinese translations.
- A heading or table whose id you write in braces as a footnote's,
  such as `## Sources {#note-1}`, keeps it, and the footnote takes
  `note-1-1`, so both links land. The footnote's marker linked to the
  heading.
- A footnote in a value shown through `markdown` has its way back
  labelled, as a page's own footnotes do.
- A call naming a snippet with a hyphen, which no call can name, is
  told. Statac published it as text with nothing said.
- An `id` that is neither six letters and digits nor a UUID is told,
  with both forms named.
- `statac search` given a site's folder alone, which it reads as the
  words to look for, says so, and its usage shows the words first.
- `page.key` on a page made from no item is told as a value the page
  hasn't got. It was told as a name not available yet.
- `numbered: true` in a folder's `index.md` says its numbering is
  meant, and Statac no longer notes the folder.
- An image a template shows with no alt text is told once at the place
  it is shown, naming every page, however each page links it. It was
  told once for each way a page linked it.
- A blank line inside a block of a template indented with tabs, before
  a block inside it, no longer publishes a stray `>` on a line of its
  own.

### Fixed in `statac import`

- It asks for each feed the old site made, at its old URL: every feed
  in a Zola site's `feed_filenames`, in Zola's order; the feed
  Eleventy's RSS plugin made, and one made with `getFeedTemplate` and
  `addTemplate`; and it counts each as kept. It writes no
  `feed: false` for a list Hugo made no feed for, and writes a
  `feed_key` only for a site whose feeds come across.
- It names the template each feed needs by the feed's own file, such
  as `templates/index.xml` for Hugo's RSS feeds, and says which of the
  docs' Atom and RSS templates to copy as which. It named
  `templates/feed.xml`, the docs' Atom template, for every feed.
- Posts keep their feed ids: it writes `feed_ids_from` with the old
  tool's name, so a post whose URL holds a space, an accented letter or
  a bracket keeps the id the old feed gave it; it writes
  `feed_ids_before` for an Eleventy site that used the RSS plugin; and
  it names each post whose id Eleventy's RSS plugin escaped twice in
  an Atom or JSON feed, which feed readers show once as new. A site
  with no RSS plugin is told that its posts' feed ids change. Under
  `feed_ids_from: jekyll`, a post with `%` in its file name now keeps
  the id Jekyll gave it, which changes its id on a site that moved
  before.
- It takes an Eleventy site's `url`, `title` and `language` from the
  `metadata` its RSS plugin was given, a Hugo site's language from
  `locale`, which took over from `languageCode` in Hugo 0.158, and a
  Hugo site in several languages' title from its main language. It
  says when a site's settings give no `url` or `title`, or an empty
  `title`, and the new site's title is made from the old folder's
  name.
- It looks for each Hugo page and tag at the URL Hugo published it at:
  runs of spaces, a space beside a hyphen, punctuation between spaces,
  a slash in a title, symbols such as `½`, marks that take space of
  their own under `removePathAccents`, a tag under a `[permalinks]`
  pattern, a tag's title in capitals under `:slug` and `:title`, and a
  tag whose name starts with a space. It brings across only the page
  Hugo published where Hugo read two names as one, and makes one tag
  of names Hugo made one tag of. It names each Hugo tag's feed beside
  the tag's page wherever a pattern put it.
- It keeps the URL Zola gave a page or tag whose name it spelled in
  ASCII under `[slugify]`, so `日本.md` stays at `/ri-ben/`; keeps a
  `#` under `paths = "safe"`, as Zola does; keeps the id Zola gave each
  heading, writing it after the heading in braces where Statac's own
  would differ; and says that Zola's footnote ids aren't kept.
- It takes only the folders above a Jekyll site's `_posts` as a post's
  categories. A post in `_posts/2019/` was given the category `2019`
  and a second `2019` in its URL.
- It leaves the `tags` in an Eleventy folder's data file out of its
  pages' tags, since Eleventy reads them as collections, and publishes
  no page at a folder whose `index.md` it writes only to hand down the
  folder's data.
- It gives a page no `permalink` at an old URL the new site can't
  publish at, such as `/posts/p#q/`, so the new site builds, and lists
  each such URL once, among those that can't be kept, with why. It
  lists a list's or a feed's URL that the old built site doesn't have
  under a heading of its own, with no `redirect_from` to add, where it
  advised one.
- It says when the old site's built output isn't there to check the
  old URLs against, naming the folder the old tool builds into. It
  said every page landed where the old site published it.
- It follows no symbolic link in the old site, and lists each. A page
  whose front matter Statac's YAML reader stops at comes across as
  written, listed in `IMPORT.md`, and `statac check` shows the error.
  It sends an old paginated page of the home page to the page of
  `content/posts/` with the same number.
- It lists, rather than reproduces, what Statac has no equal for: Zola's
  other `sort_by` orders, `paginate_reversed` and `render_emoji`; a
  Jekyll site's `_config.toml`, which it no longer reads, and its moved
  folders; Hugo's `uglyURLs` lists, moved tags, `cascade`,
  `[frontmatter]`, `enableGitInfo` and folder settings; and the rarer
  ways an Eleventy site makes its feeds and pages. `IMPORT.md` says
  what the new site needs before it builds, and calls another tool's
  shortcodes shortcodes.

## 0.6.2 - 2026-10-21

### New

- A sharing card's text can be drawn in fonts of its own, such as
  `title.font`, written as the card's `font` is: one file or a list,
  tried in order. A text without one is drawn in the card's fonts, and
  a card whose texts name none is drawn as before.

### Fixed

- Search reads a word with a typographic apostrophe, as Statac's
  typography writes it in prose, as the same word typed with a plain
  one: `doesn't` finds `doesn’t`, and `file` finds `file’s`.
- While a reader types, the last word ranks pages as one word across
  the longer words it starts, so `templ` puts a page titled Templates
  first. A rare longer word it starts put the pages holding it first.

## 0.6.1 - 2026-10-21

### Fixed

- Search ranks a page whose title or headings hold the words above a
  shorter page that only mentions them. Only the length of a part's
  text counts against a word now; a short part won by being short.
- A named pipe or other special file in `content/`, `assets/`,
  `static/`, `data/` or `templates/` is refused as not a file. The
  build waited on it.
- A link swapped in while a build runs is refused, never followed: in
  the site's folders, in what Statac keeps in `.statac/` between
  builds, and in the spare copy of the built site there. Reads, writes
  and removals followed it, and in the spare copy could reach outside
  the site.
- A name in raw HTML that the site has no snippet for, such as
  `@nope(a &amp; b)`, is published as it was written. Its `&` and tags
  were escaped again and shown as text.
- A value shown through `markdown` shows the character references in a
  call to a name the site has no snippet for as the characters they
  stand for, as it shows the rest of the value.
- A page's opening words and word count read `&#0;`, a surrogate or a
  number past Unicode as a replacement character, and `&#128;` to
  `&#159;` as the characters a browser shows for them, and tell
  `&AMP;` from `&Amp;` as a browser does.
- A macro, `set` block, `call` or `filter` keeps a line its body has
  apart from the next where a branch not taken stood between them, as a
  page's own lines do.
- A template's lines, and a macro's or `set` block's body where it is
  written, are indented by their markup read as a browser reads it. A
  comment closed by `--!>` had spaces added inside a `<pre>` after it,
  and an apostrophe in an attribute's value written without quotes, as
  `title=it's`, or a `<command>`, left the lines after it indented
  wrongly.
- A template that made two values Statac refused and then used the
  first is told the first's problem. It could be told the second's.
- A template giving a number with a fraction to `format`'s `%d` is told
  the mark needs a whole number. It was told it needed a number.
- `statac new` and `statac import` quote a value such as `yes` or `no`,
  so that other tools reading the site's YAML don't take it as true or
  false, and escape U+FFFE and U+FFFF, which a site's own settings
  couldn't be read back with.

### Changed

- A template reading a long data file on every page, such as
  `site.data.links | length`, no longer reads through the whole file
  each time, so such a build no longer slows as the file grows.

## 0.6.0 - 2026-10-20

### New

- Search, Statac's own. A page that reads its search box's `index`,
  given by `page.search_box()`, has the build publish a search index
  under `/search/`, and Statac's script for reading it, which ranks as
  `statac search` does. A search finds the other forms of a word the
  site holds (`running` finds `run`) in 31 languages, ignores accents,
  reads Chinese and Japanese by word however the reader's browser
  divides them, searches each language of a site apart, names and dates
  each result as the site's lists do, shows the text round the words at
  the part of the page that matched, and offers the nearest words the
  site holds when it finds nothing.
- A new site has a search page at `/search/`, linked from the menu,
  and a snippet that makes the box, for the author to place elsewhere.
  `statac new --update --missing` adds them to an existing site.
- `statac search "words"` shows the pages a reader of the site would
  find for the words, best first, each at the part of the page that
  matched, with the text round the words. `--drafts` searches drafts
  and posts waiting for their day too, and `--language` the pages in
  one language.
- `search: false` leaves a page out of search, and `pages.search: false`
  a folder's pages.
- Serve's panel has a Search tab: type some words to see the pages a
  reader would find, best first, drafts and posts waiting for their day
  among them, and why each ranks where it does.
- `search_without_javascript: duckduckgo` in `statac.yaml` has the
  starter's search page, for a reader without JavaScript, send the
  search to DuckDuckGo limited to the site's host. Without it, the page
  says search needs JavaScript.

### Fixed

- Markup in the writing after a comment ended by `--!>` is kept, where
  it was left out of the page, and so is a comment written inside an
  element whose text is raw, such as `<xmp>`.
- `@url(...)`, snippet calls and typography after a comment in the
  writing ended by `--!>`, or written `<!-->` or `<!--->`, work. They
  were left as typed, and the address told as a broken link.
- `@url(...)` and snippet calls after `<!--` written inside a
  `<textarea>`, `<title>`, `<style>` or another element whose text holds
  no markup work. They were left as typed, and the address told as a
  broken link.
- A link in the writing whose unquoted address ends in `/`, as
  `<a href=https://example.org/>`, no longer gets a second link inside
  it, round an address in its text.
- A `<pre>` written in capitals, a `<textarea>`, `<listing>` or `<xmp>`
  in a list item or quotation keeps its lines as written. They gained
  the item's indentation.
- A heading holding a `<script>` or `<style>` element keeps that
  element's text out of its id, its entry in `page.toc` and a title
  made from it: `## Setup <script>x()</script>` has the id `setup` and
  the entry "Setup". The element's text was taken in. The id of such a
  heading changes, so a link written with its old id stops landing;
  give the heading an id in braces to keep one.
- A broken link written in a template whose text holds U+0001 or U+0002
  is told at the address or expression that wrote it. Such a character
  was taken for one of the template's own tags, and the link could be
  told at the wrong place.
- A component call written `{{ <name /> }}` in the writing, as Zola 0.23
  writes it, is warned of as published as text.
- `statac import` reads a site made with Zola 0.23 or later as Zola
  did, running its writing as a template: `{% raw %}` and
  `{% endraw %}` are taken out and what they held kept, and the
  writing's other template code, such as `{{ page.title }}`, is listed
  in the report rather than passed over. A file
  `skip_content_templating` names is left as written.
- `statac import` converts Zola 0.23's component calls,
  `{{ <name /> }}` and `{% <name> %}` to `{% </name> %}`, as it converts
  shortcodes, and lists those it can't.
- `statac import` gives a Zola 0.23 site's `{{ get_url(...) }}` the
  address Zola gave it, and lists a call to one of Zola's other
  functions in the report, rather than making either a call to a
  snippet that doesn't exist.
- `statac import` links a Zola 0.23 site's `@/` link to a file beside a
  page, in the writing or through `get_url`, to the file where it is
  published. It was left as written and reported as a page that didn't
  come across.
- `statac import` mends a post's front matter where a quoted value
  needs it, though the front matter holds a character from Unicode's
  private use area, as icon fonts use. Such a post was left for the
  reader to refuse.

### Changed

- A site whose pages take their titles from their opening headings,
  with no `title:` of their own, builds with about 5% less work: each
  page's writing is read once, where it was read again for its title
  wherever another page named it.
- A build no longer opens each picture, recording, font and PDF
  published as it is to look for JavaScript, which no browser runs from
  them.

## 0.5.1 - 2026-10-19

### Fixed

- `statac import` from Hugo or Zola keeps a character from the start of
  Unicode's private use area, as icon fonts use, where the post has it:
  in its words, its HTML, its code and the values of a tag it converts.
  It was left out of the new post without a word, and could change what
  was made of the writing round it, such as a blank line put inside a
  block of HTML.
- On a site in several languages, an author's page standing at a tag's
  or a series' address, as `content/tags/rust.md` at `/tags/rust/`, is
  the translation of the page Statac makes for that tag or series in
  another language, and each names the other with `hreflang`. Neither
  was linked to the other. Where a page in another language stands at
  the author's page's own path, the two are still each other's
  translations.
- A page's `permalink`, written on it or handed down to it, can use a
  field declared in `statac.yaml`, as `/books/{isbn}/` with
  `fields.isbn: text`, filled from the page's own value as the pages
  `each:` makes fill it from their items. So can an item's own
  `permalink`. A page without the field stops the build, as
  `` `content/a.md` has no `isbn` for `{isbn}` ``. The pattern stopped
  with "there is no piece `{isbn}`" on every page.
- A `permalink` piece naming a nested field, as `{book.author}` for
  `author:` written beneath `book:`, is filled from that value, on a
  page and in the pattern of a page with `each:` alike. Such a page or
  item was told it had no `book.author`, however the field was written.
- A post made from a data file by a page with `each:` that comes to be
  published at another address, with no `id:` of its own, is told as
  `feed-id-changed`, as a post in a file of its own is, with the `id:`
  line to add to its item in the data file. Its feed id changed with
  nothing said, so feed readers showed it again as new. An item with a
  name is known by its name, and an item in a list by its fields
  written exactly as before. `write_feed_ids` never writes into a data
  file, so such a post always takes the line by hand.
- A feed keeps a span in a post whose class starts `hl-`, as
  `<span class="hl-keyword">own</span>`, as the author wrote it. The
  feed took the span out and kept only its words, as it takes out the
  markup that colours code.
- `page.words` and `page.opening`, and an excerpt taken from the
  author's own `<p>`, read the author's HTML as a browser does. A `>`
  inside a quoted attribute, as in `<abbr title="a > b">`, or inside a
  comment ended the tag early, so the rest was counted as words and
  could show in the opening or the excerpt. A `<` that opens no tag, as
  in `<p>1 < 2</p>`, hid the words up to the next `>`, and `<pre` or
  `<p>` inside an attribute was taken for the element.
- `page.words` and `page.opening` leave out the text of the author's
  `<script>`, `<style>` and `<title>`, and of `<iframe>`, `<noembed>`,
  `<noframes>` and `<noscript>`, which a browser doesn't show. They
  counted it as words, and a script inside the opening paragraph showed
  in the opening. The text of a `<textarea>`, which a reader sees, is
  still counted.
- A page showing coloured code only in another page's excerpt, as a
  list of posts whose excerpts hold a block of code, warns
  `code-colours-not-loaded` where no template asks for `code.css`, at
  the first block the excerpt shows. It said nothing, and the code
  showed without its colours.
- A snippet handed a value through more than four snippets in turn is
  checked, when the templates are read, against everything it may be
  handed. It was checked against only what came through the first four,
  so a mistake in it could go untold until a page was built through it.
- A value a snippet passes on as it was handed, as `{{ two(x=x) }}`, is
  checked as what it is however many snippets pass it on, so a field
  misspelt in the third snippet a page reaches, or one further on, is
  told when the templates are read. From the third snippet on, the
  value was taken as anything, and the mistake went untold until a page
  was built through it.
- A page published where a list's later page goes, as
  `content/blog/page/2.md` where page 2 of `/blog/` goes, is told at
  `later_pages` in `statac.yaml`, with the page's first line shown
  beneath, or at the page where the site doesn't set `later_pages`. It
  was told in no file.
- `serve` shows what a page shows of a file through `page.file()`,
  beside the page or in `static/`, as the file is after a save made
  while a build ran. Where the disk's times move in steps, it does so
  too after a save at the file's length straight after a build read it,
  which left the file its time. Serve kept the page as it was until the
  file changed again. A file `before_build` writes again in that way is
  taken as the command's writing, where serve took it for a save of the
  author's.

## 0.5.0 - 2026-10-18

### Fixed

- A branch in the body of a `macro`, a `{% set %}` block, a
  `{% call %}` or a `{% filter %}` keeps the lines round it apart,
  whether or not it is written, where a tag alone on its line opens or
  closes it, as `<p>a</p>{% if x %}` with `{% endif %}` on the line
  after the branch, or `{% if x %}` on the line after `<p>a</p>` with
  `<p>b</p>{% endif %}`. The line before the branch ran into the line
  after it where the branch wasn't written, or the branch's last line
  ran into the line after it where it was.
- The body of a `macro`, or of a `{% set %}` block, written by its call
  or its name alone on an indented line, as `  {{ card() }}`, starts at
  that line's indent, its lines keeping their indent one to another. It
  kept the indent its lines had in the template on top of the line's,
  as a body indented under its tag inside a `<div>`. Written from a line
  at the left, the body is written as typed.
- What a `{% call %}` or a `{% filter %}` writes, where its tag stands
  alone on an indented line outside `<pre>`, `<textarea>`, `<script>`
  and `<style>`, starts at that line's indent, its lines keeping their
  indent one to another. A filter's lines kept the indent they had in
  the template in place of the line's, and a call's started at the
  left. Where the tag is at the left, or shares its line, what it writes
  is written as typed.
- A value written alone on an indented line inside the body of a
  `macro`, a `{% call %}`, a `{% filter %}` or a `{% set %}` block, such
  as `  {{ card() }}` or `  {{ caller() }}`, lines up at that line's
  indent as it does elsewhere, with no blank line added after it. A
  body written by `caller()` gives up the indent its lines have under
  the `{% call %}` tag. A line in such a body that comes to nothing, as
  `  {% if x %}<p>b</p>{% endif %}` where `x` is false, is left out, where
  it left a line of spaces.
- A loop whose tag stands alone on its line and whose end tag closes
  the last line of its body, as `  <li>{{ i }}</li>{% endfor %}` on the
  line after `{% for i in items %}`, writes each pass on lines of its
  own where that body is indented. The passes ran together on one line,
  each with the body's indent. Where the body is at the left, or inside
  `<pre>`, `<textarea>`, `<script>` or `<style>`, or the template writes
  plain text, the passes run together as typed, since a line ending
  between them could change the page.
- A line in the body of a `macro`, a `{% set %}` block, a `{% call %}`
  or a `{% filter %}` whose tags at its end close two branches, as
  `<p>x</p>{% endif %}{% endif %}`, keeps the line after it apart. It
  ran into the line after it where it was written.
- "Ran `after_build`, which added N files and changed M" counts a file
  the command wrote again at its length straight after the build wrote
  it. Where the disk's times move in steps, as by a few thousandths of a
  second on Linux, such a file kept its time and went uncounted.
- `serve` shows a file of the site, such as one under `static/`, saved
  again at its length straight after a build copied it, from the next
  build on. Where the disk's times move in steps, such a file kept its
  time, and serve kept the copy from before the save until the file
  changed again.
- A site's file published at `.statac-clock`, at the top of the output,
  stays there in a build that runs `after_build`. The build removed it
  and counted it as removed by the command.
- `serve` shows a folder's `pager` as it is once the folder's page
  stops being published, or is published again. It kept the pager as
  it was until something else built the page again.
- `serve` builds again a file of the site moved out of it and back
  unchanged, as with the Bin's Put Back. It took the file's return as
  no change and served the site without it until it was next saved.
- `serve` says nothing where the system loses its reports of changes and
  a look finds nothing changed, as serve's own writing on a large site
  can cause as it starts. It said "Files changed" and built nothing.
- The panel `serve` adds to each page shows each name, such as a field
  or a file, in a code type, as its terminal picks names out. It showed
  the backticks round them.
- A template that can't be read, named by `extends` with a name worked
  out as the template runs, as `{% extends "ho" ~ "me" %}`, is told
  once, where it can't be read. A page built through it also said the
  template doesn't exist.
- A template that sorts by the same field twice is told that pages were
  sorted last at each sort that put some last. Both were told at the
  first.
- A picture with no description, or a HEIC picture, that a template
  asks for with `page.photo` is told at the call that asked for it,
  once, naming every page built through it. Where the template asked
  for more than one picture, it was told at the page with no line of
  the template, and where it asked for one, once for each page.
- A problem with a value shown through `highlight` or `markdown` is told
  at the filter that showed it, with the caret under the whole of what
  it was handed. Where a template used the filter on two values worked
  out as it ran, both were told at the first, and the caret left out an
  opening bracket.
- `webmention()` called more than once on a site without webmentions is
  told at a call. It was told at the page, with no line of the template.
- A character from the start of Unicode's private use area, as icon
  fonts use, is published where it is written. In HTML it stopped the
  build with a fault of Statac's own, and elsewhere a few of them
  together could be published as something else written on the page,
  such as the address an `@url(...)` stands for.

### Changed

- The panel `statac serve` adds to each page is a tab at the foot of the
  page, showing the counts of problems and when the site was last built,
  and saying when a build runs or stops. A build that stops opens a card
  of what stopped it, with the lines and the fix. Opened, the panel is a
  drawer of the problems, the names a template can write, this page and
  the site, which can be resized, docked to the side or kept as a bar
  across the page. It is light or dark as the system is, opens and closes
  with Alt+Shift+S, and keeps its settings in the browser.
- The panel opens a file it names at its line in VS Code, VS Code
  Insiders, Cursor, Zed or Sublime Text, as its settings choose, and
  copies the file and line as `file:line` for any other editor.
- `serve` builds again only the pages a change to a page's front matter
  can show on, where a template chooses or orders pages by a field, as
  the starter's menu does. A post's new title built every page again;
  on a site of 10,000 posts it now builds 5 to 7, in about 0.2 seconds
  against 0.55.
- `serve` builds again only the pages a tag added to a post, or a post
  added or removed, can show on, where every page shows the site's
  folders, its feed, its home and a menu chosen from its pages, as the
  starter's do. Each built every page again; on a site of 10,000 posts a
  tag no other post has now builds under 10 pages, one other posts have
  about 550, and a post added or removed 700 to 1,600, each in about
  0.25 seconds against 0.55.
- `serve` reads again only the pages' files that changed since its last
  build, where it can tell, in place of every file at each save. On a
  site of 10,000 posts a save of one post's text takes about 0.14
  seconds against 0.2. Not on Windows, where Statac can't tell.
- `serve` on macOS builds a save made after a pause as promptly as one
  made straight after another, in about 0.2 seconds on a site of 10,000
  posts against up to 1.6. macOS tells of a change late once nothing has
  been written to the disk for a while, so serve makes and removes a
  file in `.statac` every 10 seconds.

## 0.4.0 - 2026-10-17

### New

- `statac new --no-starter` and `statac import --no-starter` make a site
  without the starter's templates, snippets, stylesheet and pages, for an
  author who writes their own: it holds `statac.yaml` and `.gitignore`,
  besides what an import brings. `statac new` asks this last of its
  questions, so answers piped in for the other four still answer them.
  `statac new --update` says once that a site has none of the starter's
  files, in place of naming each, and `--missing` writes
  `data/words.yaml` where it isn't there, with the other files the site
  lacks.
- A `permalink` written on a folder's own `index.md`, the home page's
  included, publishes the page and its list at that address, as it does
  any other page, and the list's later pages and its feed follow it. It
  was set aside, the page staying at the folder's address. A `permalink`
  handed down from a folder above still leaves a folder's own page where
  it is, and the folder's other files stay at the folder's address.
- `statac import` keeps a list at the address the old site gave it,
  where that isn't its folder's, as with a Hugo section's `url` or a
  pattern in `permalinks.section`, or a Zola folder whose name
  Statac spells otherwise: the folder's `index.md` carries that address
  as its `permalink`, and the list's later pages and its feed follow it.
  The list stood at its folder's address, sending readers on from the old
  one, and a list Hugo made with no `_index.md` was left with nothing at
  its old address.
- A table takes an id and classes from a line of braces directly under
  its last row, as `{#speed .comparison}`, written as a heading's are,
  and Statac writes them on its `<table>`. Braces holding anything else,
  or escaped as `\{`, stay a row of the table, as they were. An id given
  twice, on a table or a heading, is told as a heading's always was.
- A site can write tables its own way with `snippets/markdown/table.html`,
  handed `head`, the heading row's cells, and `rows`, each a list of
  cells, every cell with its `body` and its column's `align`, besides
  the table's `id` and `classes`. Without the file, Statac writes a
  table as it does now.

### Fixed

- `statac import` from Hugo looks for each list's later pages at the
  addresses Hugo gave them, and sends readers on from the first,
  `/posts/page/1/`, to the list. A list under `uglyURLs` stays at its
  folder's address, where Hugo has published it since version 0.148,
  and sends readers on from `/posts.html`, where earlier versions did,
  unless the old site's built output has no such file; without the built
  output, the report says that nothing answers those versions' later
  pages, `/posts/page/2.html`. Neither kind of later page was looked for
  or named.
- `statac import` sends readers on from each numbered page of a list or
  a tag that it finds in the old site's built output, such as
  `/posts/page/2.html` or `/tags/rust/page/1/`, to the new list's page of
  the same number. From Hugo, it also looks for a tag's later pages where
  Statac publishes them, and under `uglyURLs` sends readers on from a
  tag's page, such as `/tags/rust.html`, and from `/tags.html`, where
  Hugo before 0.148 listed every tag. Nothing answered these.
- `statac import` from Hugo sends readers on to each tag's page and
  each series' from the address `[permalinks]` gave it, where Statac has
  no pattern to match, as `/tags/rust/posts/`, and from each of its
  later pages it finds in the old site's built output; without the
  built output, the report names one of those later pages, which
  nothing answers. The old address was only listed as one nothing
  answered, and the later pages were never named. Under `uglyURLs`, a
  series' page, such as `/series/rust.html`, sends readers on too, where
  nothing answered it.
- A code block opened with a caption in quotes and no language, as
  `` ```"An example" ``, is no longer warned of as a language Statac has
  no colours for. The block was built rightly; only the warning was
  wrong.
- A broken link a template writes from a value, as `href="{{ address }}"`,
  is told at the line of the template that wrote it wherever the line
  is. It was told at the page alone, or at another line, where the line
  was inside a loop, opened a block or came after the page's writing.
- A broken link a snippet writes from a value is told at the snippet's
  line, whether the writing calls the snippet or a template does. It was
  told where the template shows the page's writing, or at another line
  of the snippet.
- Why Sass stopped is told in Statac's own words, with a suggestion
  where there is a good one: `` Sass stopped: no variable called `$link` ``
  where it was `Sass stopped: undefined variable`. What an `@error` in
  the site's Sass says is quoted exactly, where it lost its capital and
  its full stop. A value CSS has no way to write, as `1px * 1px`, is
  told by the stylesheet alone where Sass gives no place for it. It was
  told at the stylesheet's first line.
- What stops a template is told in Statac's own words, with a
  suggestion where there is a good one: `` Malformed template: `{% if %}`
  is never closed ``, with `` Close it with `{% endif %}`. ``, where it
  was `Malformed template: unexpected end of input, expected end of
  block`, and `` Template stopped: can't use `+` on text and a number ``.
- A template that can't be read is told once, at the template. It was
  told again at each template that extends it and at each call of it.
- Where the template engine gives a line but no place, as for a
  `{% block main required %}` that nothing fills, what stopped it is
  told at its tag on that line. It was told at the page alone.
- `{% extends "base.html" %}` takes the template `base`, as
  `layout: base.html` does. It was told as a template that doesn't
  exist.
- A feed carries code as written, with its language's class, and
  without the markup that colours it, since a feed reader loads no
  stylesheet to colour it with.
- Haxe is coloured after a string in single quotes. The rest of the
  line was left uncoloured, and later lines could lose colours too.
- Builds of one site run one at a time. A build started while another
  runs, such as `statac build` beside `statac serve`, says that it is
  waiting and builds once the other has finished. Two at once could stop
  with `Can't write`, leave no built site, or warn `cache-not-written`.
- A division by zero in a template, with `/`, `//` or `%`, stops the
  template, told at the sum. `/` wrote `inf` or `nan` on the page and
  nothing was said.
- A minus before a field, as `{{ -page.date }}`, makes the field's value
  negative, and stops the template at it where the value isn't a
  number. It was told as an unknown field called `-page.date`.
- A function that takes no names, handed one, as `url(to="/about/")`,
  is told as having no argument called it. It was told as handed a
  value that isn't an address.
- The lines of a block indented under its tag only to read the
  template, as a loop's body under `{% for %}` level with the line
  before it, are written at the indent of the lines round them. They
  were written that much further in. A body the markup indents, one
  step inside the `<ul>` its tag stands level with, is written as typed,
  as is any body where Statac can't tell which was meant. Pages look
  the same; only their source changes.
- A block's body written in place of another template's `{% block %}`
  lines up where it lands: its lines give up the indent they have in
  common and take the indent of the line the `{% block %}` tags share,
  or of the body they replace. It was written as typed, wherever that
  left it. The body `{{ super() }}` writes lines up at that line's
  indent, and a `{% block %}`'s own body gives up indent it has only for
  the template's sake, as a loop's does. Each deeper level of a
  recursive loop goes in as far as its `{{ loop() }}` stands inside its
  `{% for %}` tag, where only the level's first line moved.
- A line that opens with a tag and holds more than tags, as
  `{% if page.summary %}<p>{{ page.summary }}</p>{% endif %}`, keeps its
  indent, and keeps its line end if it also closes with a tag and a
  later line writes something. Both were left out, so the line was
  written at the left and ran into the next. A line whose tags write
  nothing between them is still not written.
- A line that holds more than tags and closes with one, as
  `<p>{{ page.title }}</p>{% if page.summary %}` or a recursive loop's
  `</ul>{% endif %}`, keeps its line end where a later line writes
  something, whether or not it opens with a tag, inside `<pre>` and
  `<script>` as anywhere. It was left out, so the next line ran into
  it. Where the branch after such a line isn't written, as an
  `{% if %}` that is false or a `{% for %}` over nothing, the tag
  closing it on a line of its own ends the line instead, which ran
  into the line after the branch.
- Two `statac serve` sessions of one site at once each build into a
  folder of their own, the second into `.statac/serve-2`. Both built
  into `.statac/serve`, so each could show pages the other wrote, such
  as drafts where only one was showing them.
- `statac serve` says when the system keeps it from answering the
  browser, as when no file handles are left: "Can't answer the browser:
  too many open files." It says so once, and again only after it has
  answered in between. It dropped the connection with nothing said.
- `statac serve` answers each request a program sends on one connection
  before the first is answered. It answered the first alone.
- The bar `statac serve` adds to each page, and its details, keep the
  size the reader's browser gives text, whatever font size the site sets
  on the page's `html`. A site setting `html { font-size: 62.5% }` drew
  them at about 9 pixels.
- `loop` in a loop's own `if`, as in `{% for x in xs if loop.first %}`,
  is told as an unknown name when the template is read, at the `loop`,
  with `` `loop` is set only inside the loop: test it with an `{% if %}`
  there. `` It was accepted, and the build stopped at the first page
  with no value for it, suggesting a `default` that stopped it again.

### Changed

- The template language follows Jinja more closely, so a template may
  write some things differently:
  - `title` starts a word with a capital only after a space, a hyphen or
    an opening bracket: `node.js` becomes `Node.js` where it became
    `Node.Js`, and `at&t` becomes `At&t` where it became `At&T`.
  - `round` takes a value halfway between two to the even one:
    `2.5 | round` is `2.0`, where it was `3.0`.
  - `//` and `%` round down for a negative divisor: `7 // -2` is `-4`
    and `7 % -2` is `-1`, where they were `-3` and `1`.
  - A list or map written on a page as it stands quotes its text in
    single quotes, `['a']`, where it used double.
  - `true` and `false` count as numbers to `is number` and `sum`, and
    the `upper` and `lower` tests pass only text with a letter in it.

## 0.3.0 - 2026-10-12

### New

- `code_style: none` turns the colouring of code off: code is published
  as written with its language's class, no `code.css` is published,
  `css("code.css")` writes nothing, and a language Statac has no colours
  for is not warned of. For a site that colours code in the browser, or
  wants it plain.
- A note, `loop-over-every-page`, where a template loops over a list of
  the whole site's pages on so many pages that the build's work grows
  with the square of the site. `check` shows the loop and, where the loop
  keeps pages by a field, the filter to write in its place, which Statac
  works out once for the site.

### Fixed

- A text in quotes in front matter, settings or data that runs onto a
  line one space deeper than its field is read, as YAML reads it, where
  it was refused as a quote never closed. A later line no deeper than
  the field is told at that line, with the spaces that mend it.
- A large site whose layout chooses pages from `site.pages` by a field,
  as the starter's menu does with `selectattr("menu")`, builds in a
  fraction of the time: 10,000 posts in 2 seconds where 0.2.0 took 17.
- A large site whose layout asks for a list of pages on every page and
  takes a few of them, or counts them, as `site.posts[:5]` and
  `tag.posts | length` do, no longer builds slower with the square of
  its size: with 8,000 posts, 1 second where it took 3 to 10.
- A large site whose layout looks through every page on every page no
  longer takes memory with the square of its size: with 8,000 posts,
  under 1 GB where it took 4 GB, and 39 GB where it sorted them.
- A large site whose layout chooses from the site's posts or pages on
  every page, or puts them in order, with `tagged`, `in`, `where`,
  `selectattr`, `rejectattr`, `sort` or `newest_first`, no longer builds
  slower with the square of its size: with 8,000 posts, 1 second where
  it took 6 to 90.
- `related(page)`, which the starter's posts show three of, no longer
  slows a large site by the square of its size: 32,000 posts build in
  3.7 seconds where they took 6.9.

### Changed

- Coloured code is marked with the kind of each piece, such as
  `hl-keyword` or `hl-comment`, in place of a numbered class, and is
  marked the same whatever `code_style` is. `code.css` gives each kind
  one rule, and a site changes a colour by setting its property, such as
  `--code-comment`, in a stylesheet of its own. Rules a site wrote for
  the numbered classes no longer apply.
- Code is coloured by another library, so some pieces take another
  colour than before, and Statac no longer builds the Oniguruma and
  Zstandard C libraries or carries code under the EUPL.
- A site whose own `snippets/markdown/code.html` never shows `body`
  builds faster: Statac no longer colours code that file can't show.

## 0.2.0 - 2026-10-11

### New

- A new site has a page listing every tag, at `/tags/`, with an "All
  tags" link from each tag's page, and an About page to adopt or copy.
- A page joins the starter's menu with `menu: true` in its front matter.
- HTML in the writing that is left open, or closes what it never opened,
  is warned of at its tag and published as written.
- `site.webmentions` tells a template whether the site takes webmentions.
- An install script for Linux and macOS:
  `curl -fsSL https://get.statac.dev | sh`.

### Fixed

- `statac new --update` and `check` take a file that no longer resembles
  any version of the starter's as the author's own, and say nothing of a
  newer one.
- `check` no longer stops a template that only pages with `needs: url`
  use from reading the site's address.
- `check` no longer lists a file as writing JavaScript where the page's
  scripts all come from another file.
- A site in a right-to-left language Statac has no words for is built
  right to left.
- A tag whose `>` starts the next line of HTML is read without a fault.
- A more marker inside an element the writing's HTML holds open is
  refused.
- The starter's snippets treat `pictured=false` and `nested=false` as
  not given.
- A line of many unclosed `$` or bare addresses no longer slows a build.
- Importing a Hugo site keeps its feeds where `outputs` and
  `outputFormats` put them, leaves a page's own fields beneath `params`
  where they bear one of Hugo's names, and speaks of the later pages
  wherever the site has more posts than a page shows.
- Importing a Zola site answers a list's `/page/1/` from the list's own
  page.

### Changed

- What `statac new --update` shows of a changed file, the explanations
  of problems and the opening lines of each starter file are reworded.

## 0.1.0 - 2026-10-10

The first release.
