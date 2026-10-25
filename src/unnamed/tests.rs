use super::unnamed;
use crate::problem::{Kind, LabelledBy};

/// What `html` is told of, each with the text its tag opens with.
fn told(html: &str, titled: bool) -> Vec<(Kind, Option<&str>)> {
    (unnamed(html, titled).into_iter())
        .map(|found| (found.kind, found.at.map(|at| &html[at])))
        .collect()
}

fn link(to: Option<&str>, image: bool) -> Kind {
    Kind::MissingLinkText {
        href: to.map(str::to_owned),
        image,
    }
}

#[test]
fn a_link_or_a_button_with_no_words_is_told_at_its_tag() {
    let html = "<a href=\"/a/\"></a>\
        <a href=\"/b/\">B</a>\
        <a href=\"/c/\"><img src=\"c.png\" alt=\"\"></a>\
        <a href=\"/d/\"><img src=\"d.png\" alt=\"D\"></a>\
        <a href=\"/e/\" aria-label=\"E\"><svg></svg></a>\
        <a href=\"/f/\"><svg viewBox=\"0 0 1 1\"><title>F</title><path d=\"M0 0\"/></svg></a>\
        <a href=\"/g/\"><span aria-hidden=\"true\">G</span></a>\
        <a href=\"/h/\" tabindex=\"-1\" aria-hidden=\"true\"><img src=\"h.png\" alt=\"\"></a>\
        <a name=\"i\"></a>\
        <a href=\"\"> &nbsp; </a>\
        <a href=\"/j/\">&hellip;</a>\
        <a href=\"/k/\"><script>document.write('K')</script></a>\
        <A HREF=\"/l/\" TITLE=\"L\"></A>\
        <template><a href=\"/m/\"></a></template>\
        <button></button>\
        <button type=\"submit\"><span class=\"icon\"></span>Send</button>";
    assert_eq!(
        told(html, false),
        [
            (link(Some("/a/"), false), Some("<a")),
            (link(Some("/c/"), true), Some("<a")),
            (link(Some("/g/"), false), Some("<a")),
            (link(None, false), Some("<a")),
            (link(Some("/k/"), false), Some("<a")),
            (Kind::MissingButtonText { image: false }, Some("<button"),),
        ]
    );
}

#[test]
fn a_link_left_open_is_told_where_another_opens_or_the_page_ends() {
    assert_eq!(
        told(
            "<a href=\"/a/\"><a href=\"/b/\">B</a><a href=\"/c/\">",
            false
        ),
        [
            (link(Some("/a/"), false), Some("<a")),
            (link(Some("/c/"), false), Some("<a")),
        ]
    );
}

#[test]
fn an_iframe_with_no_title_is_told_with_what_it_shows() {
    let html = "<iframe src=\"https://example.org/video/1\"></iframe>\
        <iframe src=\"https://example.org/video/2\" title=\"Video 2\"></iframe>\
        <iframe srcdoc=\"&lt;p&gt;\"><a href=\"/a/\"></a></iframe>\
        <div aria-hidden=\"true\"><div><iframe src=\"/b/\"></iframe></div></div>\
        <noscript><iframe src=\"/c/\"></iframe></noscript>";
    let iframe = |src: Option<&str>| Kind::MissingIframeTitle {
        src: src.map(str::to_owned),
    };
    assert_eq!(
        told(html, false),
        [
            (iframe(Some("https://example.org/video/1")), Some("<iframe")),
            (iframe(None), Some("<iframe")),
        ]
    );
}

#[test]
fn a_form_control_with_no_label_is_told_with_what_labels_one_of_its_kind() {
    let html = "<input name=\"q\">\
        <label>Q <input name=\"q2\"></label>\
        <label for=\"q3\">Q</label><input id=\"q3\">\
        <input id=\"q4\"><label for=\"q4\">Q</label>\
        <input id=\"q5\">\
        <input type=\"hidden\" name=\"h\">\
        <input type=\"SUBMIT\">\
        <input type=\"button\">\
        <input type=\"button\" value=\"Go\">\
        <input type=\"image\" src=\"go.png\">\
        <input aria-label=\"Q\">\
        <li><input type=\"checkbox\" disabled> Done</li>\
        <select name=\"sort\"><option>A</option></select>\
        <textarea name=\"t\"><a href=\"/a/\"></a></textarea>";
    let label = |control: &str, name: Option<&str>, id: Option<&str>, by| Kind::MissingLabel {
        control: control.into(),
        name: name.map(str::to_owned),
        id: id.map(str::to_owned),
        by,
    };
    assert_eq!(
        told(html, false),
        [
            (
                label("input", Some("q"), None, LabelledBy::Label),
                Some("<input")
            ),
            (
                label("input", None, Some("q5"), LabelledBy::Label),
                Some("<input")
            ),
            (
                label("input", None, None, LabelledBy::Value),
                Some("<input")
            ),
            (label("input", None, None, LabelledBy::Alt), Some("<input")),
            (
                label("select", Some("sort"), None, LabelledBy::Label),
                Some("<select")
            ),
            (
                label("textarea", Some("t"), None, LabelledBy::Label),
                Some("<textarea")
            ),
        ]
    );
}

#[test]
fn a_document_with_no_title_is_told_at_its_head_where_it_should_have_one() {
    let missing = |empty| Kind::MissingTitle { empty };
    let html = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"></head>\
        <svg><title>Logo</title></svg>";
    assert_eq!(told(html, true), [(missing(false), Some("<head"))]);
    assert_eq!(told(html, false), []);
    assert_eq!(told("<html><p>A", true), [(missing(false), Some("<html"))]);
    assert_eq!(
        told("<!DOCTYPE html>\n<p>A", true),
        [(missing(false), Some("<!DOCTYPE"))]
    );
    // A piece of a page is no document.
    assert_eq!(told("<!-- A --><p>A", true), []);
    assert_eq!(
        told("<head><title> &#32; </title>", true),
        [(missing(true), Some("<title"))]
    );
    assert_eq!(told("<title>Post &amp; 1</title>", true), []);
}
