//! What each of Statac's settings and front matter fields does, in a line
//! or two, as an editor's hover says it, with the section of the docs
//! that says more.

/// What one of Statac's settings or fields does, and where the docs say
/// more.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Described {
    pub text: &'static str,
    pub more: &'static str,
}

const fn described(text: &'static str, more: &'static str) -> Described {
    Described { text, more }
}

const SITE: &str = "https://statac.dev/settings/#site";
const DATES: &str = "https://statac.dev/settings/#dates";
const WRITING: &str = "https://statac.dev/settings/#writing-and-code";
const LISTS: &str = "https://statac.dev/settings/#lists";
const PICTURES: &str = "https://statac.dev/settings/#pictures";
const FEEDS: &str = "https://statac.dev/settings/#feeds";
const LINKS: &str = "https://statac.dev/settings/#links";
const HOOKS: &str = "https://statac.dev/settings/#hooks";

const PAGE: &str = "https://statac.dev/front-matter/#page-fields";
const ADDRESS: &str = "https://statac.dev/front-matter/#address-fields";
const LIST: &str = "https://statac.dev/front-matter/#list-fields";
const LIST_PAGE: &str = "https://statac.dev/front-matter/#list-page-fields";
const SHARING: &str = "https://statac.dev/front-matter/#sharing-fields";

/// Where the docs say more of a cascade line.
pub(crate) const HANDING_DOWN: &str = "https://statac.dev/front-matter/#handing-down";

/// Where the docs say more of a setting for one language.
pub(crate) const FOR_ONE_LANGUAGE: &str =
    "https://statac.dev/dates-and-languages/#settings-for-one-language";

/// Where the docs say more of a custom setting.
pub(crate) const OWN_VALUE: &str = "https://statac.dev/settings/#your-own";

/// Where the docs say more of a custom field.
pub(crate) const OWN_FIELD: &str = "https://statac.dev/front-matter/#your-own";

/// Each of Statac's settings, by name.
const SETTINGS: &[(&str, Described)] = &[
    (
        "title",
        described(
            "The site's name, which templates show as `site.title`. Feeds need it.",
            SITE,
        ),
    ),
    (
        "url",
        described(
            "The site's full URL, starting with `https://` or `http://`. Feeds, social \
             cards, webmentions and pages with `needs: url` are left out until it is set.",
            SITE,
        ),
    ),
    (
        "language",
        described(
            "The site's language as a code, such as `en`, `en-GB` or `fr`. A site in \
             several lists them all, the main one first: `[ar, en]`. Unset, it is `en`.",
            SITE,
        ),
    ),
    (
        "timezone",
        described(
            "The time zone your clock is in, such as `Europe/London`. Dates with no \
             offset are read in it, and it decides which day is today. Unset, it is UTC.",
            SITE,
        ),
    ),
    (
        "date_format",
        described(
            "How dates look, written as an example date: `31 December 2025`. Unset, \
             dates look as the site's language usually writes them.",
            DATES,
        ),
    ),
    (
        "date_zeros",
        described(
            "Where the month is written in figures, `false` drops the leading zero from \
             the day and the month: `4/3/2026`, not `04/03/2026`. Unset, it is `true`.",
            DATES,
        ),
    ),
    (
        "typography",
        described(
            "`false` leaves quotes, `--`, `---` and `...` in the content exactly as \
             typed. Unset, it is `true`.",
            WRITING,
        ),
    ),
    (
        "code_style",
        described(
            "The colours of code in the content: `ocean`, `gruvbox`, `solarized`, \
             `catppuccin` or `one-half`, or `none` to colour no code. Unset, it is \
             `ocean`.",
            WRITING,
        ),
    ),
    (
        "posts_per_page",
        described(
            "How many posts each page of a list shows, the rest going on to paginated \
             pages. A list page can set its own number. Unset, or `0`, a list shows all \
             its posts on one page.",
            LISTS,
        ),
    ),
    (
        "pagination.permalink",
        described(
            "Where a list's paginated pages go: a pattern that starts with `{list}`, \
             holds `{n}` once and ends with `/`. Unset, it is `{list}page/{n}/`.",
            LISTS,
        ),
    ),
    (
        "tags.permalink",
        described(
            "Where each tag's list page is published: a pattern ending with `{slug}/`. \
             Unset, it is `/tags/{slug}/`.",
            LISTS,
        ),
    ),
    (
        "series.permalink",
        described(
            "Where each series' list page is published: a pattern ending with \
             `{slug}/`. Unset, it is `/series/{slug}/`.",
            LISTS,
        ),
    ),
    (
        "tags.pages",
        described(
            "`true` makes a list page for each tag where no page of yours is published \
             at its URL. The tags work in templates either way. Unset, it is `false`.",
            LISTS,
        ),
    ),
    (
        "series.pages",
        described(
            "`true` makes a list page for each series where no page of yours is \
             published at its URL. The series work in templates either way. Unset, it is \
             `false`.",
            LISTS,
        ),
    ),
    (
        "tags.slugs",
        described(
            "The slug for a tag: each tag on its own line beneath the setting, as in \
             `C++: cpp`.",
            LISTS,
        ),
    ),
    (
        "series.slugs",
        described(
            "The slug for a series: each series on its own line beneath the setting, as \
             in `C++: cpp`.",
            LISTS,
        ),
    ),
    (
        "max_width",
        described(
            "The widest resized copy Statac makes of an image, in pixels: a whole \
             number, 1 or more. Unset, it is `1600`.",
            PICTURES,
        ),
    ),
    (
        "image_sizes",
        described(
            "How wide images are shown on the page, written as the `sizes` of an \
             `<img>`. Unset, it is `(max-width: 46rem) 100vw, 46rem`.",
            PICTURES,
        ),
    ),
    (
        "optimise_images",
        described(
            "`false` stops Statac making resized copies of images: each is published as \
             it is. Unset, it is `true`.",
            PICTURES,
        ),
    ),
    (
        "feed",
        described(
            "`true` gives the site's own list a feed, beside its page as `feed.xml`, and a \
             name or a URL puts it there. A list of them gives it a feed at each, templates \
             linking the first. On a site in several languages, each language's own list has \
             one. A `feed:` line on the list page wins over it.",
            FEEDS,
        ),
    ),
    (
        "feed_key",
        described(
            "What each post's id in a feed is made from. `statac new` writes a new one. \
             Once a feed is published, never change it.",
            FEEDS,
        ),
    ),
    (
        "feed_ids_before",
        described(
            "A date, for a site moving from another tool. Each post dated before it \
             keeps its full URL as its id in a feed.",
            FEEDS,
        ),
    ),
    (
        "feed_ids_from",
        described(
            "`jekyll`. With `feed_ids_before`, the posts dated before it keep the ids \
             that tool's feed gave them.",
            FEEDS,
        ),
    ),
    (
        "write_feed_ids",
        described(
            "`true` makes `statac serve` and `statac stamp` write an `id:` line into \
             each post of a feed that `statac build` hasn't yet published from this \
             computer. Unset, it is `false`.",
            FEEDS,
        ),
    ),
    (
        "webmentions",
        described(
            "`true` means the site receives webmentions. They need `url`. Unset, it is \
             `false`.",
            "https://statac.dev/settings/#webmentions",
        ),
    ),
    (
        "refuse_webmentions_from",
        described(
            "Full URLs whose webmentions are left out: one sent from a page at or under a \
             URL listed, or by a sender whose own URL is. `statac refresh` deletes their \
             files for good.",
            "https://statac.dev/settings/#webmentions",
        ),
    ),
    (
        "search_without_javascript",
        described(
            "`duckduckgo` gives a reader whose browser runs no JavaScript a box on the \
             search page that sends the search to DuckDuckGo, limited to the site's \
             host. Their searches go to another company.",
            "https://statac.dev/settings/#search",
        ),
    ),
    (
        "quiet",
        described(
            "The kinds of warning and note that `statac check` no longer shows, such as \
             `[broken-link]`. `build` and `check` still count them. Errors can't be \
             quietened.",
            "https://statac.dev/settings/#quiet",
        ),
    ),
    (
        "before_build",
        described(
            "A hook run before the site is read, such as a command that makes a \
             stylesheet.",
            HOOKS,
        ),
    ),
    (
        "before_build_watches",
        described(
            "The files and folders the `before_build` command reads, so `statac serve` \
             runs it again when they change. Folders end with `/`: `[styles/]`.",
            HOOKS,
        ),
    ),
    (
        "after_build",
        described(
            "A hook run on the built files after the site builds with no errors, before \
             they are published.",
            HOOKS,
        ),
    ),
    (
        "after_build_outputs",
        described(
            "The files and folders the `after_build` command makes, from the top of the \
             built site: `[indexer/]`. Links into them count as working links.",
            HOOKS,
        ),
    ),
    (
        "other_hosts",
        described(
            "Other hosts the site is published at, such as `[www.example.org]`. A link \
             to one of them is checked as a link into the site.",
            LINKS,
        ),
    ),
    (
        "served_elsewhere",
        described(
            "Paths on the site's host that something else publishes, such as \
             `[/video/]`. Links into them aren't checked.",
            LINKS,
        ),
    ),
    (
        "exclude",
        described(
            "Names in `content/` that are never published. Setting it replaces the whole \
             list. Unset, it is `.DS_Store`, `Thumbs.db` and `desktop.ini`.",
            LINKS,
        ),
    ),
];

/// A line declaring a custom field.
const FIELDS: Described = described(
    "The kind of value a custom field holds, with `required` before it where \
     every page must have it: `fields.isbn: required text`.",
    "https://statac.dev/settings/#fields",
);

/// A line declaring a field for the pages of one type.
const TYPES: Described = described(
    "The kind of value a custom field holds on the pages of one `type`, with \
     `required` before it where each of them must have it: `types.book.isbn: required \
     text`.",
    "https://statac.dev/settings/#fields",
);

/// Each of Statac's front matter fields, by name.
const FIELDS_OF_A_PAGE: &[(&str, Described)] = &[
    (
        "title",
        described(
            "The page's title. Without it, the page's opening `# heading` is its title.",
            PAGE,
        ),
    ),
    (
        "untitled",
        described(
            "`true` where the page is meant to have no title, such as a short note. \
             Statac refers to it by its date instead.",
            PAGE,
        ),
    ),
    ("summary", described("A line about the page.", PAGE)),
    (
        "date",
        described(
            "The page's date, year first: `2026-10-04`, or `2026-10-04 09:30` with a \
             time. `=today` has `statac serve` or `statac stamp` write the day in.",
            PAGE,
        ),
    ),
    (
        "updated",
        described("When the page last changed, written as a `date` is.", PAGE),
    ),
    (
        "draft",
        described(
            "`true` leaves the page out of the build. `statac serve` still shows it, \
             marked as a draft.",
            PAGE,
        ),
    ),
    (
        "layout",
        described(
            "The template the page is built with, by its name in `templates/`. On a \
             page with `each:`, the template for the pages it makes.",
            PAGE,
        ),
    ),
    (
        "type",
        described(
            "What kind of page this is, in a word of your own. Where `templates/` has a \
             template of that name and the page sets no `layout`, it is built with that \
             template.",
            PAGE,
        ),
    ),
    (
        "permalink",
        described(
            "The page's URL, or a pattern such as `/{year}/{slug}/`.",
            ADDRESS,
        ),
    ),
    (
        "redirect_from",
        described("Old URLs of the page. Each one redirects to it.", ADDRESS),
    ),
    (
        "redirect_to",
        described(
            "The URL the page redirects to. The page publishes nothing else.",
            ADDRESS,
        ),
    ),
    (
        "needs",
        described(
            "`needs: url` leaves the page out until the site has a `url`.",
            ADDRESS,
        ),
    ),
    (
        "translation_key",
        described(
            "Links the page to its translations where their names differ. Pages with \
             the same key are translations of each other.",
            ADDRESS,
        ),
    ),
    (
        "tags",
        described(
            "The page's tags, as a list in brackets: `[Rust, Go]`.",
            LIST,
        ),
    ),
    (
        "series",
        described("The one series the page is part of.", LIST),
    ),
    (
        "neighbours",
        described(
            "Which list `page.previous` and `page.next` move through: `site`, `folder`, \
             `series`, or `false` for none.",
            LIST,
        ),
    ),
    (
        "posts",
        described(
            "A list of the page's own, chosen from all the posts: \
             `site.posts | tagged(\"Rust\")`.",
            LIST_PAGE,
        ),
    ),
    (
        "each",
        described(
            "Makes a page for each item of a table in `data/`, as in `each: \
             data.people`, or for each post, as in `each: site.posts`.",
            LIST_PAGE,
        ),
    ),
    (
        "order",
        described(
            "The list's order: `newest first`, `oldest first`, or the names of its pages \
             in the order you want.",
            LIST_PAGE,
        ),
    ),
    (
        "numbered",
        described(
            "In a folder's `index.md`, `false` where the numbers at the start of its \
             pages' names are part of the names.",
            LIST_PAGE,
        ),
    ),
    (
        "posts_per_page",
        described(
            "How many posts each page of the list shows. `0` shows them all on one page.",
            LIST_PAGE,
        ),
    ),
    (
        "index_page",
        described(
            "`false` publishes nothing at the list's URL, and none of its paginated \
             pages. The list still works in templates.",
            LIST_PAGE,
        ),
    ),
    (
        "feed",
        described(
            "`true` gives the list a feed, beside its page as `feed.xml`, and a name or a \
             URL puts it there. A list of them gives it a feed at each, templates linking the \
             first. A list has no feed unless it asks for one.",
            LIST_PAGE,
        ),
    ),
    (
        "id",
        described(
            "What identifies a post in a feed, even if the post moves.",
            SHARING,
        ),
    ),
    (
        "search",
        described("`false` leaves the page out of the site's search.", SHARING),
    ),
    (
        "card",
        described(
            "Which of the site's social cards is drawn for the page, once cards are \
             turned on.",
            SHARING,
        ),
    ),
    (
        "og_image",
        described("The page's social image, in place of a card.", SHARING),
    ),
    (
        "og_title",
        described("The page's title on a card and in a link preview.", SHARING),
    ),
    (
        "mentions",
        described(
            "`true` where the page shows its webmentions. A template reads it as \
             `page.mentions`.",
            SHARING,
        ),
    ),
];

/// What the setting `name` does, where it is one of Statac's; a line
/// declaring a field, as `fields.isbn`, is described as all such lines are.
pub(crate) fn setting(name: &str) -> Option<Described> {
    if name
        .strip_prefix("fields.")
        .is_some_and(|rest| !rest.is_empty())
    {
        return Some(FIELDS);
    }
    if (name.strip_prefix("types.")).is_some_and(|rest| rest.contains('.')) {
        return Some(TYPES);
    }
    find(SETTINGS, name)
}

/// What the front matter field `name` does, where it is one of Statac's.
pub(crate) fn field(name: &str) -> Option<Described> {
    find(FIELDS_OF_A_PAGE, name)
}

fn find(described: &[(&str, Described)], name: &str) -> Option<Described> {
    (described.iter()).find_map(|(said, described)| (*said == name).then_some(*described))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::words::PageWord;

    /// Every description, by the word it describes.
    fn all() -> impl Iterator<Item = (&'static str, Described)> {
        let lines = [("fields.<name>", FIELDS), ("types.<type>.<name>", TYPES)];
        (SETTINGS.iter().chain(FIELDS_OF_A_PAGE).copied()).chain(lines)
    }

    #[test]
    fn every_setting_and_word_is_described_once() {
        let settings: Vec<&str> = SETTINGS.iter().map(|(name, _)| *name).collect();
        let mut statacs: Vec<&str> = crate::config::setting_names().collect();
        statacs.sort_unstable();
        let mut described = settings.clone();
        described.sort_unstable();
        assert_eq!(described, statacs);
        let mut words: Vec<&str> = FIELDS_OF_A_PAGE.iter().map(|(name, _)| *name).collect();
        let mut statacs: Vec<&str> = PageWord::all().map(PageWord::name).collect();
        words.sort_unstable();
        statacs.sort_unstable();
        assert_eq!(words, statacs);
        assert_eq!(setting("fields.book.author"), Some(FIELDS));
        assert_eq!(setting("types.book.isbn"), Some(TYPES));
        assert_eq!(setting("types.book"), None);
        assert_eq!(setting("fields."), None);
    }

    #[test]
    fn descriptions_follow_the_wording_guide() {
        for (name, described) in all() {
            let text = described.text;
            assert!(text.ends_with('.'), "{name}");
            assert!(
                text.starts_with(|c: char| c.is_uppercase() || c == '`'),
                "{name}"
            );
            assert!(!text.contains(['\u{2013}', '\u{2014}', '!']), "{name}");
            assert!(!text.contains("  "), "{name}");
            assert!(described.more.starts_with("https://statac.dev/"), "{name}");
            // A word in code, such as `url`, is the author's to write.
            let prose: Vec<&str> = text.split('`').step_by(2).collect();
            let lower = prose.join(" ").to_lowercase();
            let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
            for banned in [
                "invalid",
                "please",
                "simply",
                "just",
                "parse",
                "render",
                "undefined",
            ] {
                assert!(!words.contains(&banned), "{name} says {banned:?}");
            }
        }
    }
}
