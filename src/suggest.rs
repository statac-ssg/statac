//! Did you mean: the nearest of a few known spellings to one written.

/// The candidate nearest `written`, ignoring case, if it is near enough to
/// be what was meant: about one slip in every three characters. The first
/// candidate wins a tie.
pub(crate) fn nearest<'a>(
    written: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let written = written.to_lowercase();
    let limit = (written.chars().count() / 3).max(1);
    candidates
        .into_iter()
        .map(|candidate| (distance(&written, &candidate.to_lowercase()), candidate))
        .filter(|&(d, _)| d <= limit)
        .min_by_key(|&(d, _)| d)
        .map(|(_, candidate)| candidate)
}

/// Whether `written` is `known` with one slip, ignoring case: a character
/// added, left out or changed, or two neighbours swapped.
pub(crate) fn is_one_slip(written: &str, known: &str) -> bool {
    distance(&written.to_lowercase(), &known.to_lowercase()) == 1
}

/// The candidate one slip from `written`, ignoring case: a character
/// added, left out or changed, two neighbours swapped, or capitals alone.
/// None for fewer than four characters, or where `written` is a
/// candidate as it stands.
pub(crate) fn one_slip<'a>(
    written: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    if written.chars().count() < 4 {
        return None;
    }
    let lower = written.to_lowercase();
    let mut nearest = None;
    for candidate in candidates {
        if candidate == written {
            return None;
        }
        let d = distance(&lower, &candidate.to_lowercase());
        if d <= 1 && nearest.is_none_or(|(best, _)| d < best) {
            nearest = Some((d, candidate));
        }
    }
    nearest.map(|(_, candidate)| candidate)
}

/// Edits between two spellings, a swap of neighbours counting as one.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let width = b.len() + 1;
    let mut d = vec![0; (a.len() + 1) * width];
    for (i, row) in d.chunks_mut(width).enumerate() {
        row[0] = i;
    }
    for (j, cell) in d.iter_mut().take(width).enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[(i - 1) * width + j] + 1)
                .min(d[i * width + j - 1] + 1)
                .min(d[(i - 1) * width + j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[(i - 2) * width + j - 2] + 1);
            }
            d[i * width + j] = best;
        }
    }
    d[a.len() * width + b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slips_are_counted_as_edits() {
        assert_eq!(distance("build", "build"), 0);
        assert_eq!(distance("biuld", "build"), 1);
        assert_eq!(distance("buid", "build"), 1);
        assert_eq!(distance("bulids", "build"), 2);
        assert_eq!(distance("", "check"), 5);
    }

    #[test]
    fn the_nearest_is_offered_only_when_near() {
        let commands = ["build", "check", "explain"];
        assert_eq!(nearest("biuld", commands), Some("build"));
        assert_eq!(nearest("Check", commands), Some("check"));
        assert_eq!(nearest("explian", commands), Some("explain"));
        assert_eq!(nearest("post-1", commands), None);
        assert_eq!(nearest("brokn-link", ["broken-link"]), Some("broken-link"));
    }

    #[test]
    fn a_slip_is_one_edit_whatever_the_case() {
        assert!(is_one_slip("subtitel", "subtitle"));
        assert!(is_one_slip("subtile", "subtitle"));
        assert!(is_one_slip("subtitlee", "subtitle"));
        assert!(is_one_slip("subtitke", "subtitle"));
        assert!(is_one_slip("SubTitel", "subtitle"));
        assert!(!is_one_slip("subtitle", "subtitle"));
        assert!(!is_one_slip("SUBTITLE", "subtitle"));
        assert!(!is_one_slip("sbutitel", "subtitle"));
    }

    #[test]
    fn one_slip_is_one_edit_and_no_more() {
        let words = ["title", "summary", "posts_per_page", "tags"];
        let cases = [
            ("titles", Some("title")),
            ("titl", Some("title")),
            ("titel", Some("title")),
            ("tilte", Some("title")),
            ("Title", Some("title")),
            ("TITLE", Some("title")),
            ("summery", Some("summary")),
            ("posts_per_pge", Some("posts_per_page")),
            ("sumary_", None),
            ("tirle_", None),
            ("tag", None),
            ("title", None),
            ("author", None),
        ];
        for (written, meant) in cases {
            assert_eq!(one_slip(written, words), meant, "{written}");
        }
    }
}
