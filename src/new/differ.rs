//! What changed between two texts, line by line, by the longest run of
//! lines the two share in order. Starter files are a few hundred lines,
//! so nothing quicker is needed.

/// One line of two texts compared.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Line<'a> {
    Same(&'a str),
    Left(&'a str),
    Came(&'a str),
}

/// A run of changes and the lines around them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Hunk<'a> {
    /// The number, in the newer text, of the run's first line, or of the
    /// line after where it stands for a run that opens with lines that
    /// left.
    pub line: usize,
    pub lines: Vec<Line<'a>>,
}

/// `text`'s lines, each with its line break where it has one.
pub(super) fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// How many lines one of `a` and `b` has and the other hasn't.
pub(super) fn distance(a: &[&str], b: &[&str]) -> usize {
    let (head, tail) = ends(a, b);
    let (a, b) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    // The two rows of the table held at once, along the shorter text.
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut above = vec![0usize; short.len() + 1];
    let mut row = vec![0usize; short.len() + 1];
    for line in long {
        for (j, other) in short.iter().enumerate() {
            row[j + 1] = match line == other {
                true => above[j] + 1,
                false => row[j].max(above[j + 1]),
            };
        }
        std::mem::swap(&mut above, &mut row);
    }
    a.len() + b.len() - 2 * above[short.len()]
}

/// Each line of `old` and `new` in order: one both have, one that left,
/// or one that came, the lines that left before those that came in each
/// change.
pub(super) fn diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Line<'a>> {
    let (head, tail) = ends(old, new);
    let (inner_old, inner_new) = (&old[head..old.len() - tail], &new[head..new.len() - tail]);
    let (n, m) = (inner_old.len(), inner_new.len());
    // `longest[i][j]`: the longest run shared by what follows line `i` of
    // the one and line `j` of the other.
    let mut longest = vec![0usize; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            longest[at(i, j)] = match inner_old[i] == inner_new[j] {
                true => longest[at(i + 1, j + 1)] + 1,
                false => longest[at(i + 1, j)].max(longest[at(i, j + 1)]),
            };
        }
    }
    let mut lines: Vec<Line> = old[..head].iter().map(|line| Line::Same(line)).collect();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && inner_old[i] == inner_new[j] {
            lines.push(Line::Same(inner_old[i]));
            (i, j) = (i + 1, j + 1);
        } else if i < n && (j == m || longest[at(i + 1, j)] >= longest[at(i, j + 1)]) {
            lines.push(Line::Left(inner_old[i]));
            i += 1;
        } else {
            lines.push(Line::Came(inner_new[j]));
            j += 1;
        }
    }
    lines.extend(old[head + n..].iter().map(|line| Line::Same(line)));
    lines
}

/// The changes in `lines`, each with up to `around` lines both texts have
/// on either side; changes nearer than that share a hunk.
pub(super) fn hunks<'a>(lines: &[Line<'a>], around: usize) -> Vec<Hunk<'a>> {
    let changed: Vec<usize> = (lines.iter().enumerate())
        .filter(|(_, line)| !matches!(line, Line::Same(_)))
        .map(|(at, _)| at)
        .collect();
    let mut hunks = Vec::new();
    let mut at = 0;
    while at < changed.len() {
        let first = changed[at];
        let mut last = first;
        while at + 1 < changed.len() && changed[at + 1] - last <= 2 * around + 1 {
            at += 1;
            last = changed[at];
        }
        at += 1;
        let start = first.saturating_sub(around);
        let end = (last + around + 1).min(lines.len());
        let before = lines[..start]
            .iter()
            .filter(|line| !matches!(line, Line::Left(_)))
            .count();
        hunks.push(Hunk {
            line: before + 1,
            lines: lines[start..end].to_vec(),
        });
    }
    hunks
}

/// How many lines `a` and `b` open with alike, and how many of the rest
/// they end with alike.
fn ends(a: &[&str], b: &[&str]) -> (usize, usize) {
    let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let (a, b) = (&a[head..], &b[head..]);
    let tail = (a.iter().rev().zip(b.iter().rev()))
        .take_while(|(x, y)| x == y)
        .count();
    (head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    use Line::{Came, Left, Same};

    fn compared<'a>(old: &'a str, new: &'a str) -> Vec<Line<'a>> {
        diff(&lines(old), &lines(new))
    }

    /// The text of `n` numbered lines, `line 1\n` to `line n\n`.
    fn numbered(n: usize) -> String {
        (1..=n).map(|i| format!("line {i}\n")).collect()
    }

    #[test]
    fn a_line_added_removed_or_changed_is_told() {
        assert_eq!(
            compared("a\nb\n", "a\nx\nb\n"),
            [Same("a\n"), Came("x\n"), Same("b\n")]
        );
        assert_eq!(
            compared("a\nx\nb\n", "a\nb\n"),
            [Same("a\n"), Left("x\n"), Same("b\n")]
        );
        assert_eq!(
            compared("a\nx\nb\n", "a\ny\nb\n"),
            [Same("a\n"), Left("x\n"), Came("y\n"), Same("b\n")]
        );
    }

    #[test]
    fn a_change_at_the_start_or_the_end_is_told() {
        assert_eq!(
            compared("x\na\nb\n", "y\na\nb\n"),
            [Left("x\n"), Came("y\n"), Same("a\n"), Same("b\n")]
        );
        assert_eq!(
            compared("a\nb\nx\n", "a\nb\ny\n"),
            [Same("a\n"), Same("b\n"), Left("x\n"), Came("y\n")]
        );
        assert_eq!(compared("", "a\n"), [Came("a\n")]);
        assert_eq!(compared("a\n", ""), [Left("a\n")]);
    }

    #[test]
    fn a_last_line_without_a_line_break_differs_from_one_with() {
        assert_eq!(
            compared("a\nb\n", "a\nb"),
            [Same("a\n"), Left("b\n"), Came("b")]
        );
        assert_eq!(distance(&lines("a\nb"), &lines("a\nb\n")), 2);
    }

    #[test]
    fn several_lines_changed_leave_before_they_come() {
        assert_eq!(
            compared("a\nx\ny\nb\n", "a\np\nq\nb\n"),
            [
                Same("a\n"),
                Left("x\n"),
                Left("y\n"),
                Came("p\n"),
                Came("q\n"),
                Same("b\n")
            ]
        );
    }

    #[test]
    fn the_distance_counts_the_lines_only_one_text_has() {
        let (a, b) = (lines("a\nb\nc\nd\n"), lines("a\nx\nc\nd\ny\n"));
        assert_eq!(distance(&a, &b), 3);
        assert_eq!(distance(&b, &a), 3);
        assert_eq!(distance(&a, &a), 0);
        assert_eq!(distance(&a, &[]), 4);
        let changes = compared("a\nb\nc\nd\n", "a\nx\nc\nd\ny\n");
        let differing = changes.iter().filter(|line| !matches!(line, Same(_)));
        assert_eq!(differing.count(), 3);
    }

    #[test]
    fn changes_near_each_other_share_a_hunk_and_far_apart_have_their_own() {
        let old = numbered(20);
        let near = old
            .replace("line 5\n", "five\n")
            .replace("line 10\n", "ten\n");
        let hunks = hunks(&compared(&old, &near), 3);
        assert_eq!(hunks.len(), 1, "{hunks:?}");
        assert_eq!(hunks[0].line, 2);
        assert_eq!(hunks[0].lines.first(), Some(&Same("line 2\n")));
        assert_eq!(hunks[0].lines.last(), Some(&Same("line 13\n")));

        let far = old
            .replace("line 3\n", "three\n")
            .replace("line 17\n", "seventeen\n");
        let changes = compared(&old, &far);
        let hunks = super::hunks(&changes, 3);
        assert_eq!(hunks.len(), 2, "{hunks:?}");
        assert_eq!(
            hunks[0].lines,
            [
                Same("line 1\n"),
                Same("line 2\n"),
                Left("line 3\n"),
                Came("three\n"),
                Same("line 4\n"),
                Same("line 5\n"),
                Same("line 6\n")
            ]
        );
        assert_eq!(hunks[0].line, 1);
        assert_eq!(hunks[1].line, 14);
        assert_eq!(hunks[1].lines.last(), Some(&Same("line 20\n")));
    }

    #[test]
    fn a_hunk_is_numbered_by_the_newer_texts_lines() {
        let old = numbered(10);
        let new = format!("line 0\n{}", old.replace("line 8\n", ""));
        let hunks = hunks(&compared(&old, &new), 1);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].line, 1);
        assert_eq!(hunks[0].lines, [Came("line 0\n"), Same("line 1\n")]);
        // Line 8 left: the hunk opens at 7, the eighth line of the newer.
        assert_eq!(hunks[1].line, 8);
        assert_eq!(
            hunks[1].lines,
            [Same("line 7\n"), Left("line 8\n"), Same("line 9\n")]
        );
    }

    #[test]
    fn texts_alike_have_no_hunks() {
        let text = numbered(5);
        assert!(hunks(&compared(&text, &text), 3).is_empty());
    }
}
