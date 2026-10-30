//! The pieces of the writing a site may write itself, each through a file
//! of its name in `snippets/markdown/`, and what each is handed.

use super::code_lines::line_words;
use crate::highlight::{self, FirstWord, Language};

/// A kind of element in the writing whose HTML a site may write itself.
/// Nothing calls a piece's file by name: Statac writes every such element
/// through it.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum Piece {
    Heading,
    Link,
    Image,
    /// A code block.
    Code,
    /// Code among the words.
    InlineCode,
    /// Where the writing points at a footnote.
    Footnote,
    /// The footnotes gathered at the foot of the page.
    Footnotes,
    Callout,
    Table,
}

impl Piece {
    pub(crate) const ALL: [Piece; 9] = [
        Piece::Heading,
        Piece::Link,
        Piece::Image,
        Piece::Code,
        Piece::InlineCode,
        Piece::Footnote,
        Piece::Footnotes,
        Piece::Callout,
        Piece::Table,
    ];

    /// Its file's name in `snippets/markdown/`, without `.html`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Piece::Heading => "heading",
            Piece::Link => "link",
            Piece::Image => "image",
            Piece::Code => "code",
            Piece::InlineCode => "inline_code",
            Piece::Footnote => "footnote",
            Piece::Footnotes => "footnotes",
            Piece::Callout => "callout",
            Piece::Table => "table",
        }
    }

    pub(crate) fn of_name(name: &str) -> Option<Piece> {
        Piece::ALL.into_iter().find(|piece| piece.name() == name)
    }

    /// Every name its file is handed besides `page` and `site`. The writing
    /// hands all but a link's `external`, a callout's `title` and an
    /// image's `image`, which are the site's to say.
    pub(crate) fn handed(self) -> &'static [&'static str] {
        match self {
            Piece::Heading => &["level", "id", "classes", "body"],
            Piece::Link => &["url", "title", "body", "external"],
            Piece::Image => &["url", "alt", "title", "width", "height", "image"],
            Piece::Code => &["code", "body", "language", "caption"],
            Piece::InlineCode => &["code"],
            Piece::Footnote => &["number", "shown", "id", "note"],
            Piece::Footnotes => &["notes", "label"],
            Piece::Callout => &["kind", "title", "body"],
            Piece::Table => &["head", "rows", "id", "classes"],
        }
    }

    /// The piece a file that isn't one was most likely meant as: near it
    /// in spelling, or opening with its name, as `codeblock` does `code`.
    /// Where it opens with two, as `footnotes-old` does, the longer.
    pub(crate) fn meant_by(name: &str) -> Option<Piece> {
        if let Some(near) = crate::suggest::nearest(name, Piece::ALL.map(Piece::name)) {
            return Piece::of_name(near);
        }
        let lowered = name.to_lowercase();
        Piece::ALL
            .into_iter()
            .filter(|piece| lowered.starts_with(piece.name()))
            .max_by_key(|piece| piece.name().len())
    }
}

/// A value a piece's file is handed.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum PieceValue {
    /// Words, printed escaped. Nothing to give is empty words, which print
    /// nothing and are false in an `{% if %}`.
    Text(String),
    Number(u64),
    /// HTML Statac wrote, printed as it is.
    Html(String),
    List(Vec<PieceValue>),
    /// Values by name, as each footnote is handed.
    Record(Vec<(&'static str, PieceValue)>),
}

/// A code block's language and caption, as a piece is handed them, from
/// the line opening its fence: the first word is the language, unless the
/// line opens with a quote, and the rest is the caption, without the quotes
/// round it where it is quoted whole. A first word that is a file's name
/// opens the caption, and the language is the one its name gives. The
/// words asking how its lines are shown are neither.
pub(super) fn fence(info: &str) -> (&str, &str) {
    let info = line_words(info).0.trim();
    let language = language_word(info).unwrap_or("");
    let rest = &info[language.len()..];
    let (language, rest) = match highlight::first_word(language) {
        FirstWord::File(Language::Known(name)) => (name, info),
        FirstWord::File(_) => ("", info),
        FirstWord::Language(_) => (language, rest),
    };
    let rest = rest.trim();
    let caption = ['"', '\'']
        .into_iter()
        .find_map(|quote| {
            rest.strip_prefix(quote)?
                .strip_suffix(quote)
                .filter(|inner| !inner.contains(quote))
        })
        .unwrap_or(rest);
    (language, caption)
}

/// The word of a fence's opening line that names its language or its
/// file: the first, and none where the line opens with a quoted caption
/// or holds only words asking how its lines are shown.
pub(crate) fn language_word(info: &str) -> Option<&str> {
    let info = line_words(info).0.trim_start();
    match info.starts_with(['"', '\'']) {
        true => None,
        false => info.split_whitespace().next(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_gives_its_language_and_caption() {
        assert_eq!(fence("rust"), ("rust", ""));
        assert_eq!(fence("rust example.rs"), ("rust", "example.rs"));
        assert_eq!(fence("rust \"The example\""), ("rust", "The example"));
        assert_eq!(fence("\"mermaid\""), ("", "mermaid"));
        assert_eq!(fence("\"An example\""), ("", "An example"));
        assert_eq!(fence("'An example'"), ("", "An example"));
        assert_eq!(fence(""), ("", ""));
    }

    #[test]
    fn a_fence_opening_with_a_files_name_is_captioned_with_it() {
        assert_eq!(fence("example.rs"), ("rs", "example.rs"));
        assert_eq!(
            fence("Gemfile for the site"),
            ("ruby", "Gemfile for the site")
        );
        assert_eq!(fence("notes-1.txt"), ("", "notes-1.txt"));
        assert_eq!(fence(".gitignore"), (".gitignore", ""));
        assert_eq!(fence("Makefile"), ("Makefile", ""));
    }

    #[test]
    fn a_file_that_is_no_piece_is_offered_the_one_it_was_meant_as() {
        assert_eq!(Piece::meant_by("headings"), Some(Piece::Heading));
        assert_eq!(Piece::meant_by("codeblock"), Some(Piece::Code));
        assert_eq!(Piece::meant_by("footnotes-old"), Some(Piece::Footnotes));
        assert_eq!(Piece::meant_by("inlinecode"), Some(Piece::InlineCode));
        assert_eq!(Piece::meant_by("tables"), Some(Piece::Table));
        assert_eq!(Piece::meant_by("blockquote"), None);
    }

    #[test]
    fn words_asking_how_lines_are_shown_are_no_language_or_caption() {
        assert_eq!(fence("rust main.rs numbered=true"), ("rust", "main.rs"));
        assert_eq!(fence("main.rs mark=2"), ("rs", "main.rs"));
        assert_eq!(fence("\"The example\" mark=2"), ("", "The example"));
        assert_eq!(language_word("numbered=true"), None);
    }
}
