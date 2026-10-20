//! What Statac reads of the engine by where in memory it is. Each line of
//! a template is known by where the engine holds it, and the engine writes
//! a template's own text from there; a text Statac makes is known by its
//! address, and the engine hands that text on, not a copy. Where either
//! stops being so nothing fails: lines are written as typed and texts are
//! placed at the template. So a change to the engine fails here instead.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};
use std::{fmt, io};

use minijinja::{Environment, Held, Value, context};

use super::*;
use crate::source::{Place, Sources, Span};

thread_local! {
    /// Whether what is written now is a value's, not a template's own text.
    static VALUE: Cell<bool> = const { Cell::new(false) };
    /// A template's own text as written, by where in memory it was written
    /// from.
    static OWN: RefCell<Vec<(usize, String)>> = const { RefCell::new(Vec::new()) };
}

fn wrote(text: &str) {
    if !VALUE.get() {
        OWN.with_borrow_mut(|own| own.push((text.as_ptr() as usize, text.to_owned())));
    }
}

/// What a template renders into.
struct Out;

impl io::Write for Out {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        wrote(std::str::from_utf8(bytes).expect("templates write text"));
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// What a body the engine holds back renders into.
struct Body(String);

impl fmt::Write for Body {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        wrote(text);
        self.0.push_str(text);
        Ok(())
    }
}

impl Held for Body {
    fn finish(self: Box<Self>) -> String {
        self.0
    }
}

#[test]
fn the_engine_writes_a_templates_own_text_from_the_text_it_holds() {
    let base = "<main>\n  {% block main %}\n  <p>parent's block</p>\n  {% endblock %}\n</main>\n";
    let page = "{% extends \"base.html\" %}\n\
                {% macro card() %}\n  <p>macro's body</p>\n  {{ caller() }}\n{% endmacro %}\n\
                {% block main %}\n\
                {% for each in [1] %}\n  <p>loop's body</p>\n  {% endfor %}\n\
                {% set kept %}<p>set block's body</p>{% endset %}\n{{ kept }}\n\
                {% filter upper %}<p>filter block's body</p>{% endfilter %}\n\
                {% call card() %}\n  <p>call block's body</p>\n  {% endcall %}\n\
                {% raw %}<p>raw block's body {{ }}</p>{% endraw %}\n\
                {{- super() -}}\n\
                {% endblock %}\n";
    let mut env = Environment::new();
    env.set_syntax(check::rendering());
    // Every value is written through the formatter.
    env.set_formatter(|out, state, value| {
        VALUE.set(true);
        let written = minijinja::escape_formatter(out, state, value);
        VALUE.set(false);
        written
    });
    env.set_held(|| Box::new(Body(String::new())));
    env.add_template_owned("base.html", base.to_owned())
        .unwrap();
    env.add_template_owned("page.html", page.to_owned())
        .unwrap();
    let template = env.get_template("page.html").unwrap();
    template.render_captured_to((), &mut Out).unwrap();
    let held = ["base.html", "page.html"].map(|name| {
        let template = env.get_template(name).unwrap();
        let start = template.source().as_ptr() as usize;
        start..start + template.source().len()
    });
    let own = OWN.take();
    for (from, text) in &own {
        let inside = held
            .iter()
            .any(|source| *from >= source.start && from + text.len() <= source.end);
        assert!(inside, "{text:?} is written from a copy of the template");
    }
    for written in [
        "<main>",
        "parent's block",
        "macro's body",
        "loop's body",
        "set block's body",
        "filter block's body",
        "call block's body",
        "raw block's body {{ }}",
    ] {
        assert!(
            own.iter().any(|(_, text)| text.contains(written)),
            "{written:?} isn't written as the template's own text: {own:?}"
        );
    }
}

/// Notes each body it is written and gives it back in capitals.
struct Shouting {
    text: String,
    seen: Arc<Mutex<Vec<String>>>,
}

impl fmt::Write for Shouting {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.text.push_str(text);
        Ok(())
    }
}

impl Held for Shouting {
    fn finish(self: Box<Self>) -> String {
        self.seen.lock().unwrap().push(self.text.clone());
        self.text.to_uppercase()
    }
}

#[test]
fn a_body_the_engine_holds_back_is_written_into_the_writer_it_is_given() {
    let source = "{% macro m(a) %}<p>{{ a }}</p>{% endmacro %}\
                  {% set s %}<i>{{ m('x') }}</i>{% endset %}[{{ s }}]";
    let env = Environment::new();
    assert_eq!(env.render_str(source, ()).unwrap(), "[<i><p>x</p></i>]");

    let mut env = Environment::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let held = Arc::clone(&seen);
    env.set_held(move || {
        Box::new(Shouting {
            text: String::new(),
            seen: Arc::clone(&held),
        })
    });
    assert_eq!(env.render_str(source, ()).unwrap(), "[<I><P>X</P></I>]");
    assert_eq!(*seen.lock().unwrap(), ["<p>x</p>", "<i><P>X</P></i>"]);
}

#[test]
fn a_text_handed_through_the_engine_is_the_one_statac_made() {
    let built = site(&[]);
    frame::push(frame::Frame::new(Arc::clone(&built.renderer.shared), None));
    let mut sources = Sources::default();
    let place = Place {
        source: sources.add("", "Words".into()),
        span: Span::new(0, 5),
    };
    let text = frame::sourced("Words", place);
    let mut env = Environment::new();
    // The built-in filters as Statac wraps them.
    guard::install(&mut env);
    env.add_filter("placed", move |value: Value| {
        match frame::origin(&value) == Some(place) {
            true => "placed",
            false => "lost",
        }
    });
    let context = context! { text => text.clone(), map => context! { text } };
    for (how, source) in [
        ("handed to a filter", "{{ text | placed }}"),
        ("set", "{% set kept = text %}{{ kept | placed }}"),
        (
            "looped over",
            "{% for each in [text] %}{{ each | placed }}{% endfor %}",
        ),
        (
            "handed to a macro",
            "{% macro m(given) %}{{ given | placed }}{% endmacro %}{{ m(text) }}",
        ),
        ("read from a map", "{{ map.text | placed }}"),
        (
            "handed on by a filter",
            "{{ text | default(\"\") | placed }}",
        ),
    ] {
        let rendered = env.render_str(source, &context).unwrap();
        assert_eq!(rendered, "placed", "a text {how} isn't the one made");
    }
    frame::pop();
}
