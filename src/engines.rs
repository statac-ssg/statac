//! Engines on other sites that search this one alone, for a reader whose
//! browser runs no JavaScript: a plain form sends them the words.

/// An engine a form can send a search of the site to, and how the form
/// is written for it.
#[derive(Debug)]
pub(crate) struct Engine {
    /// The word `search_without_javascript` names it by.
    pub(crate) setting: &'static str,
    pub(crate) name: &'static str,
    /// Where the form sends the words.
    pub(crate) action: &'static str,
    /// The name the reader's words are sent under.
    pub(crate) words: &'static str,
    /// The name the site's host is sent under.
    pub(crate) site: &'static str,
}

const ENGINES: [Engine; 1] = [
    // DuckDuckGo adds `site:` and what `sites` holds to the words, and
    // keeps them on the text-only version it sends a reader without
    // JavaScript to.
    Engine {
        setting: "duckduckgo",
        name: "DuckDuckGo",
        action: "https://duckduckgo.com/",
        words: "q",
        site: "sites",
    },
];

/// Every word `search_without_javascript` takes.
pub(crate) const SETTING: &[&str] = &{
    let mut names = [""; ENGINES.len()];
    let mut at = 0;
    while at < names.len() {
        names[at] = ENGINES[at].setting;
        at += 1;
    }
    names
};

/// The engine `setting`, a word `search_without_javascript` takes, names.
pub(crate) fn engine(setting: &str) -> Option<&'static Engine> {
    ENGINES.iter().find(|engine| engine.setting == setting)
}

/// What an engine is told to keep to for the site at `url`, a full
/// address: its host, as `example.org` for `https://example.org/blog/`.
/// Its path is left out: an engine that ignores a path in `site:` may
/// find nothing at all, where one kept to the host finds the site's pages
/// among its neighbours'.
pub(crate) fn site(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(str::to_owned)
}
