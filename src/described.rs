//! What each of Statac's settings, front matter fields, template names,
//! filters and functions is, in a line or two, read from the files in
//! `descriptions/`: an editor's hover and completion say it, the debug
//! panel shows it beside each name, and the docs list it.
//!
//! Each file names its group in a `#` heading and says what the group is
//! in the paragraph after it. Each name follows as a `##` heading holding
//! it in backticks, its description, and a `More:` line with the docs'
//! URL where the docs say more.

use std::sync::LazyLock;

macro_rules! groups {
    ($($group:ident $name:literal,)*) => {
        /// The names described in one file of `descriptions/`.
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub enum Group {
            $($group,)*
        }

        impl Group {
            pub const ALL: &[Group] = &[$(Group::$group,)*];

            /// Its file's name in `descriptions/`, without `.md`.
            pub fn name(self) -> &'static str {
                match self {
                    $(Group::$group => $name,)*
                }
            }

            fn text(self) -> &'static str {
                match self {
                    $(Group::$group => include_str!(concat!("../descriptions/", $name, ".md")),)*
                }
            }
        }
    };
}

groups! {
    Settings "settings",
    Fields "fields",
    Handed "handed",
    Page "page",
    Site "site",
    List "list",
    Date "date",
    Webmentions "webmentions",
    Video "video",
    Image "image",
    File "file",
    Card "card",
    Heading "heading",
    Search "search",
    SearchField "search-field",
    Loop "loop",
    Filters "filters",
    Tests "tests",
    Functions "functions",
}

/// One name, and what it is.
#[derive(PartialEq, Eq, Debug)]
pub struct Described {
    pub name: String,
    /// In a sentence or two, with names in backticks.
    pub text: String,
    /// Where the docs say more.
    pub more: Option<String>,
}

/// One file, read.
struct Read {
    title: String,
    about: String,
    described: Vec<Described>,
}

static READ: LazyLock<Vec<Read>> = LazyLock::new(|| Group::ALL.iter().map(|&g| read(g)).collect());

impl Group {
    fn read(self) -> &'static Read {
        let at = (Group::ALL.iter()).position(|&group| group == self);
        &READ[at.expect("every group is listed")]
    }

    /// What the group is called, as its file's heading has it.
    pub fn title(self) -> &'static str {
        &self.read().title
    }

    /// What the group's file says of it before the first name.
    pub fn about(self) -> &'static str {
        &self.read().about
    }

    /// Every name the group describes, in its file's order.
    pub fn described(self) -> &'static [Described] {
        &self.read().described
    }

    /// What `name` is, where the group describes it.
    pub fn of(self, name: &str) -> Option<&'static Described> {
        (self.described().iter()).find(|described| described.name == name)
    }
}

/// The lines of `group`'s file read as a heading, a paragraph and the
/// names after it. A line ends at a line break of either kind, so a
/// checkout's line endings don't matter.
fn read(group: Group) -> Read {
    let file = group.name();
    let mut lines = group.text().lines();
    let title = (lines.next())
        .and_then(|line| line.strip_prefix("# "))
        .unwrap_or_else(|| panic!("descriptions/{file}.md opens with no heading"));
    let mut about: Vec<&str> = Vec::new();
    let mut described: Vec<Described> = Vec::new();
    let mut text: Vec<&str> = Vec::new();
    for line in lines {
        if let Some(heading) = line.strip_prefix("## ") {
            let name = (heading.strip_prefix('`'))
                .and_then(|name| name.strip_suffix('`'))
                .unwrap_or_else(|| panic!("descriptions/{file}.md: {heading} isn't in backticks"));
            close(&mut described, &mut text);
            described.push(Described {
                name: name.to_owned(),
                text: String::new(),
                more: None,
            });
        } else if let Some(more) = line.strip_prefix("More: ") {
            let last = described.last_mut();
            let last =
                last.unwrap_or_else(|| panic!("descriptions/{file}.md: More: before a name"));
            last.more = Some(more.trim().to_owned());
        } else if !line.trim().is_empty() {
            match described.is_empty() {
                true => about.push(line.trim()),
                false => text.push(line.trim()),
            }
        }
    }
    close(&mut described, &mut text);
    Read {
        title: title.to_owned(),
        about: about.join(" "),
        described,
    }
}

/// Gives the last name read the lines of text read since its heading.
fn close(described: &mut [Described], text: &mut Vec<&str>) {
    if let Some(last) = described.last_mut() {
        last.text = text.join(" ");
    }
    text.clear();
}

/// What the setting `name` is, where it is one of Statac's; a line
/// declaring a field, as `fields.isbn` or `types.book.isbn`, is described
/// as every such line is.
pub(crate) fn setting(name: &str) -> Option<&'static Described> {
    if (name.strip_prefix("fields.")).is_some_and(|rest| !rest.is_empty()) {
        return Group::Settings.of("fields.<name>");
    }
    if (name.strip_prefix("types.")).is_some_and(|rest| rest.contains('.')) {
        return Group::Settings.of("types.<type>.<name>");
    }
    Group::Settings.of(name)
}

/// What the front matter field `name` is, where it is one of Statac's.
pub(crate) fn field(name: &str) -> Option<&'static Described> {
    Group::Fields.of(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::words::PageWord;

    /// The names `group` describes, sorted.
    fn names(group: Group) -> Vec<&'static str> {
        let mut names: Vec<&str> = group.described().iter().map(|d| d.name.as_str()).collect();
        names.sort_unstable();
        names
    }

    fn sorted<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
        let mut names: Vec<&str> = names.collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    #[test]
    fn every_name_is_described_once() {
        for &group in Group::ALL {
            let all = group.described().len();
            assert_eq!(sorted(names(group).into_iter()).len(), all, "{group:?}");
            assert!(!group.title().is_empty() && !group.about().is_empty());
        }
        let lines = [
            "fields.<name>",
            "types.<type>.<name>",
            "pages.<field>",
            "<language>.<setting>",
        ];
        let settings = crate::config::setting_names().chain(lines);
        assert_eq!(names(Group::Settings), sorted(settings));
        let fields = PageWord::all().map(PageWord::name);
        assert_eq!(names(Group::Fields), sorted(fields));
        let filters = crate::templates::filter_names();
        assert_eq!(names(Group::Filters), sorted(filters));
        let functions = crate::templates::function_names();
        assert_eq!(names(Group::Functions), sorted(functions));
        let tests = crate::templates::test_names();
        assert_eq!(names(Group::Tests), sorted(tests));
    }

    #[test]
    fn a_line_declaring_a_field_is_described_as_every_such_line_is() {
        let fields = Group::Settings.of("fields.<name>");
        let types = Group::Settings.of("types.<type>.<name>");
        assert!(fields.is_some() && types.is_some());
        assert_eq!(setting("fields.book.author"), fields);
        assert_eq!(setting("types.book.isbn"), types);
        assert_eq!(setting("types.book"), None);
        assert_eq!(setting("fields."), None);
        assert_eq!(setting("title"), Group::Settings.of("title"));
    }

    #[test]
    fn descriptions_follow_the_wording_guide() {
        for &group in Group::ALL {
            let about = std::iter::once((group.title(), group.about(), None));
            let all = (group.described().iter())
                .map(|d| (d.name.as_str(), d.text.as_str(), d.more.as_deref()));
            for (name, text, more) in about.chain(all) {
                let name = format!("{}: {name}", group.name());
                assert!(text.ends_with('.'), "{name}");
                assert!(
                    text.starts_with(|c: char| c.is_uppercase() || c == '`'),
                    "{name}"
                );
                assert!(!text.contains(['\u{2013}', '\u{2014}']), "{name}");
                assert!(!text.contains("  "), "{name}");
                assert!(
                    more.is_none_or(|more| more.starts_with("https://statac.dev/")),
                    "{name}"
                );
                // A word in code, such as `url`, is the author's to write.
                let prose = text.split('`').step_by(2).collect::<Vec<_>>().join(" ");
                assert!(!prose.contains('!'), "{name}");
                let lower = prose.to_lowercase();
                let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
                for banned in [
                    "invalid",
                    "please",
                    "simply",
                    "just",
                    "parse",
                    "render",
                    "undefined",
                    "null",
                    "context",
                    "scope",
                    "picture",
                    "photograph",
                    "address",
                ] {
                    assert!(!words.contains(&banned), "{name} says {banned:?}");
                }
            }
        }
    }
}
