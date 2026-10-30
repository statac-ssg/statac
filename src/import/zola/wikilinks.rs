//! Zola's wikilinks, `[[name]]`, `[[name|text]]` and `![[name]]`, which it
//! reads where its settings' `[markdown]` says `wikilinks = true`. Zola
//! looks a name up first as the path of a page in `content/` without its
//! `.md`, or of a file beside a page; then as one of a page's `aliases`;
//! then, where the name holds no `/`, as the last part of such a path
//! alone. A name more than one page or file answers to links nowhere.

use std::collections::HashMap;

/// What a wikilink's name leads to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Target {
    /// A page, by its path in `content/`.
    Page(String),
    /// A file beside a page, by its path in `content/`.
    File(String),
}

impl Target {
    /// The name the path of a page or file answers to: a page's without
    /// `.md`.
    fn identity(&self) -> &str {
        match self {
            Target::Page(path) => path.strip_suffix(".md").unwrap_or(path),
            Target::File(path) => path,
        }
    }
}

/// Every page and file a wikilink can name.
#[derive(Default, Debug)]
pub(super) struct Wikilinks {
    targets: Vec<Target>,
    paths: HashMap<String, Vec<usize>>,
    aliases: HashMap<String, Vec<usize>>,
    names: HashMap<String, Vec<usize>>,
}

/// A name without the slashes round it, where anything is left.
fn bare(name: &str) -> Option<&str> {
    Some(name.trim_matches('/')).filter(|name| !name.is_empty())
}

impl Wikilinks {
    /// Adds the page at `path` in `content/`, with its `aliases`.
    pub(super) fn page(&mut self, path: &str, aliases: &[String]) {
        let at = self.add(Target::Page(path.to_owned()));
        for alias in aliases.iter().filter_map(|alias| bare(alias)) {
            let found = self.aliases.entry(alias.to_owned()).or_default();
            if !found.contains(&at) {
                found.push(at);
            }
        }
    }

    /// Adds the file beside a page at `path` in `content/`.
    pub(super) fn file(&mut self, path: &str) {
        if let Some(path) = bare(path) {
            self.add(Target::File(path.to_owned()));
        }
    }

    fn add(&mut self, target: Target) -> usize {
        let at = self.targets.len();
        let identity = target.identity().to_owned();
        if let Some((_, name)) = identity.rsplit_once('/') {
            self.names.entry(name.to_owned()).or_default().push(at);
        }
        self.paths.entry(identity).or_default().push(at);
        self.targets.push(target);
        at
    }

    /// The one page or file `name` leads to, where exactly one does.
    pub(super) fn find(&self, name: &str) -> Option<&Target> {
        let name = bare(name)?;
        let found = match self.paths.get(name) {
            Some(found) => found,
            None => match self.aliases.get(name) {
                Some(found) => found,
                None if !name.contains('/') => self.names.get(name)?,
                None => return None,
            },
        };
        let first = &self.targets[*found.first()?];
        (found.iter())
            .all(|at| self.targets[*at].identity() == first.identity())
            .then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_found_by_path_then_alias_then_its_last_part() {
        let mut wikilinks = Wikilinks::default();
        wikilinks.page("blog/overview.md", &[]);
        wikilinks.page("docs/overview.md", &[]);
        wikilinks.page("about.md", &[]);
        wikilinks.page("blog/_index.md", &[]);
        wikilinks.page("_index.md", &[]);
        wikilinks.page("guides/quickstart.md", &["/start/".to_owned()]);
        wikilinks.page("archive/manual.pdf.md", &[]);
        wikilinks.file("guides/source.pdf");
        wikilinks.file("guides/manual.pdf");
        let page = |path: &str| Some(Target::Page(path.to_owned()));
        let file = |path: &str| Some(Target::File(path.to_owned()));
        // The name, and what it leads to.
        let cases = [
            ("blog/overview", page("blog/overview.md")),
            ("/docs/overview/", page("docs/overview.md")),
            ("about", page("about.md")),
            ("blog/_index", page("blog/_index.md")),
            ("quickstart", page("guides/quickstart.md")),
            ("start", page("guides/quickstart.md")),
            // A path comes before the last part of another.
            ("_index", page("_index.md")),
            // Two pages answer to it.
            ("overview", None),
            // A page is named without its `.md`.
            ("about.md", None),
            ("guides/source.pdf", file("guides/source.pdf")),
            ("source.pdf", file("guides/source.pdf")),
            ("manual.pdf", None),
            ("guides/manual.pdf", file("guides/manual.pdf")),
            ("archive/manual.pdf", page("archive/manual.pdf.md")),
            ("manual", None),
            ("guides/start", None),
            ("", None),
            ("/", None),
        ];
        for (name, leads_to) in cases {
            assert_eq!(wikilinks.find(name).cloned(), leads_to, "{name}");
        }
    }
}
