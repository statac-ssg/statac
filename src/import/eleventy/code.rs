//! What Eleventy's highlighting plugin, `@11ty/eleventy-plugin-syntaxhighlight`,
//! reads from a code block about its lines: the groups after the language
//! of a Markdown fence, `js/1,3-4`, parted by `/`, and those of the
//! `highlight` tag, `js 1,3-4`, parted by spaces. One group marks lines;
//! two mark the first as added and the second as removed; any other number
//! marks nothing. The plugin counts lines from 0.

/// What a code block's groups ask of its lines, as the plugin reads them,
/// each run counted from the block's first line as 0.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) struct Asked {
    pub(super) marked: Vec<(i64, i64)>,
    /// The lines marked as added or removed, which Statac has no equal for.
    pub(super) changed: Vec<(i64, i64)>,
}

/// The language and what is asked of the lines where `word`, the first
/// word of a fence's opening line, is in the plugin's form,
/// `js/1,3-4`. None where the word holds no `/`, or anything after the
/// first `/` other than the digits, commas and hyphens of line numbers,
/// as `text/html`: the plugin reads every such word as a language and its
/// groups, but a word like that was more likely meant as a language alone.
pub(super) fn from_fence(word: &str) -> Option<(&str, Asked)> {
    let (language, groups) = word.split_once('/')?;
    if !groups
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b',' | b'-' | b'/'))
    {
        return None;
    }
    Some((language, from_groups(groups, '/')))
}

/// The language and what is asked of the lines of the tag
/// `{% highlight js 1,3-4 %}`, from what follows its name: Liquid's words,
/// or the text in quotes Nunjucks hands. The plugin parts both at each
/// single space, so two spaces together leave an empty group.
pub(super) fn from_tag(given: &str) -> (&str, Asked) {
    match given.split_once(' ') {
        Some((language, groups)) if !groups.is_empty() => (language, from_groups(groups, ' ')),
        Some((language, _)) => (language, Asked::default()),
        None => (given, Asked::default()),
    }
}

/// The groups of `text`, parted by `delimiter`, as the plugin reads them.
fn from_groups(text: &str, delimiter: char) -> Asked {
    let groups: Vec<&str> = text.split(delimiter).collect();
    match groups.as_slice() {
        [marked] => Asked {
            marked: runs(marked),
            changed: Vec::new(),
        },
        [added, removed] => Asked {
            marked: Vec::new(),
            changed: [runs(added), runs(removed)].concat(),
        },
        _ => Asked::default(),
    }
}

/// The runs of lines one group marks, `1,3-4`, read as the plugin reads
/// them, with JavaScript's `parseInt`: a run whose first number can't be
/// read marks nothing, and a number is read as far as its digits go.
fn runs(group: &str) -> Vec<(i64, i64)> {
    let mut runs = Vec::new();
    for run in group.split(',') {
        let mut numbers = run.trim().split('-');
        let Some(first) = numbers.next().and_then(leading_number) else {
            continue;
        };
        let last = match numbers.next().filter(|last| !last.is_empty()) {
            Some(last) => match leading_number(last) {
                Some(last) => last,
                None => continue,
            },
            None => first,
        };
        if first <= last {
            runs.push((first, last));
        }
    }
    runs
}

/// The whole number `text` opens with, as `parseInt(text, 10)` reads it:
/// after any spaces and a `+`, as far as its digits go. A sign of `-`
/// can't reach here, a run's numbers being parted at each `-`.
fn leading_number(text: &str) -> Option<i64> {
    let text = text.trim_start();
    let text = text.strip_prefix('+').unwrap_or(text);
    let digits = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    match digits {
        0 => None,
        _ => Some(text[..digits].parse().unwrap_or(i64::MAX)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(marked: &[(i64, i64)], changed: &[(i64, i64)]) -> Asked {
        Asked {
            marked: marked.to_vec(),
            changed: changed.to_vec(),
        }
    }

    #[test]
    fn a_fences_word_is_read_as_the_plugin_reads_it() {
        for (word, expected) in [
            ("js/1,3", Some(("js", asked(&[(1, 1), (3, 3)], &[])))),
            ("js/1,3-4", Some(("js", asked(&[(1, 1), (3, 4)], &[])))),
            ("js/0", Some(("js", asked(&[(0, 0)], &[])))),
            ("js/1/2", Some(("js", asked(&[], &[(1, 1), (2, 2)])))),
            ("js/1/2/3", Some(("js", asked(&[], &[])))),
            ("js/", Some(("js", asked(&[], &[])))),
            ("js//", Some(("js", asked(&[], &[])))),
            ("js/3-1", Some(("js", asked(&[], &[])))),
            ("js/-1", Some(("js", asked(&[], &[])))),
            ("js/2-", Some(("js", asked(&[(2, 2)], &[])))),
            ("js/1-2-3", Some(("js", asked(&[(1, 2)], &[])))),
            ("js/,,1", Some(("js", asked(&[(1, 1)], &[])))),
            ("/1", Some(("", asked(&[(1, 1)], &[])))),
            ("js", None),
            ("text/html", None),
            ("application/ld+json", None),
            ("js/1a", None),
            ("js/1 2", None),
        ] {
            assert_eq!(from_fence(word), expected, "{word}");
        }
    }

    #[test]
    fn a_tags_words_are_read_as_the_plugin_reads_them() {
        for (given, expected) in [
            ("js", ("js", asked(&[], &[]))),
            ("js 1,3", ("js", asked(&[(1, 1), (3, 3)], &[]))),
            ("js 0 2-3", ("js", asked(&[], &[(0, 0), (2, 3)]))),
            ("js linenos", ("js", asked(&[], &[]))),
            ("js 1,3 linenos", ("js", asked(&[], &[(1, 1), (3, 3)]))),
            ("js 1 2 3", ("js", asked(&[], &[]))),
            ("js  1", ("js", asked(&[], &[(1, 1)]))),
            ("js ", ("js", asked(&[], &[]))),
            ("js 1, 3", ("js", asked(&[], &[(1, 1), (3, 3)]))),
            ("js 1,+3,4x", ("js", asked(&[(1, 1), (3, 3), (4, 4)], &[]))),
            ("js 1-x", ("js", asked(&[], &[]))),
            ("", ("", asked(&[], &[]))),
        ] {
            assert_eq!(from_tag(given), expected, "{given}");
        }
    }
}
