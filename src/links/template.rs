//! How a template's text writes a link a built page was found to hold:
//! where the attribute's value it writes ends, and whether a value it
//! prints stands inside a tag. What a `{{ ... }}`, `{% ... %}` or
//! `{# ... #}` holds is passed over, so a quote or `>` there is no HTML.

use std::ops::Range;

/// Where the attribute's value starting at `start` in `template` ends: at
/// the quote that closes it, or for one written without quotes, at the
/// space or `>` after it.
pub(super) fn value_end(template: &str, start: usize) -> usize {
    let quote = template[..start]
        .chars()
        .next_back()
        .filter(|c| matches!(c, '"' | '\''));
    let mut end = None;
    each_outside(template, start, |at, c| {
        let ends = match quote {
            Some(quote) => c == quote,
            None => c.is_ascii_whitespace() || c == '>',
        };
        if ends {
            end = Some(at);
        }
        ends
    });
    end.unwrap_or(template.len())
}

/// Whether `template` prints HTML whole at `at`, the `{{ ... }}` printing
/// a value: outside any tag, through `markdown` or `safe`, or a page's
/// `content`.
pub(super) fn prints_whole(template: &str, at: &Range<usize>) -> bool {
    let mut in_tag = false;
    each_outside(&template[..at.start], 0, |_, c| {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {}
        }
        false
    });
    let printed = &template[at.clone()];
    !in_tag
        && ["markdown", "safe", "content"]
            .iter()
            .any(|word| printed.contains(word))
}

/// Calls `each` with each character of `text` from `from` on that is
/// outside a `{{ ... }}`, `{% ... %}` or `{# ... #}`, and where it is,
/// until `each` says to stop.
fn each_outside(text: &str, from: usize, mut each: impl FnMut(usize, char) -> bool) {
    let mut at = from;
    while let Some(c) = text[at..].chars().next() {
        let close = match text[at..].get(..2) {
            Some("{{") => Some("}}"),
            Some("{%") => Some("%}"),
            Some("{#") => Some("#}"),
            _ => None,
        };
        if let Some(close) = close {
            at = text[at + 2..]
                .find(close)
                .map_or(text.len(), |end| at + 2 + end + 2);
            continue;
        }
        if each(at, c) {
            return;
        }
        at += c.len_utf8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_ends_where_its_quote_closes_it_past_what_tags_hold() {
        let template = "<a href=\"{{ \"/a/\" }}/{% if b == \"c\" %}c{% endif %}{# \" #}\" class=x>\
                        <img src={{ c }}.jpg alt=''>";
        let start = template.find("{{").unwrap();
        let end = value_end(template, start);
        assert_eq!(
            &template[start..end],
            "{{ \"/a/\" }}/{% if b == \"c\" %}c{% endif %}{# \" #}"
        );
        let start = template.find("src=").unwrap() + 4;
        assert_eq!(&template[start..value_end(template, start)], "{{ c }}.jpg");
    }

    #[test]
    fn html_is_printed_whole_outside_a_tag() {
        let template = "<div>{{ body | markdown }}</div><a href=\"{{ link | safe }}\">\
                        {{ title }}{% if a > b %}{{ page.content }}{% endif %}";
        let at = |printed: &str| {
            let start = template.find(printed).unwrap();
            start..start + printed.len()
        };
        assert!(prints_whole(template, &at("{{ body | markdown }}")));
        assert!(!prints_whole(template, &at("{{ link | safe }}")));
        assert!(!prints_whole(template, &at("{{ title }}")));
        assert!(prints_whole(template, &at("{{ page.content }}")));
    }
}
