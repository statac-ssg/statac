//! The tree's nodes.

use super::call::Call;
use crate::source::Place;

#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    pub kind: Kind,
    /// Where it is written. A block ends at its last character, not the
    /// line ending after it.
    pub(crate) place: Place,
    pub children: Vec<Node>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Kind {
    Document,
    /// `tight` where the paragraph is written without `<p>`: the writing
    /// of a tight list item or definition.
    Paragraph {
        tight: bool,
    },
    /// `id` is the author's own where braces end the heading
    /// (`## Title {#id .class}`), and None until heading ids are given
    /// where they don't.
    Heading {
        level: u8,
        id: Option<String>,
        classes: Vec<String>,
    },
    Quotation,
    /// `> [!NOTE]` and the other kinds GitHub has.
    Callout(CalloutKind),
    /// `start` for a numbered list.
    List {
        start: Option<u64>,
    },
    Item,
    DefinitionList,
    Term,
    Definition,
    CodeBlock {
        info: String,
        text: String,
        fenced: bool,
    },
    /// Holds `Html` lines and the calls among them.
    HtmlBlock,
    Rule,
    /// `id` and `classes` are the author's where a line of braces follows
    /// the last row (`{#id .class}`).
    Table {
        alignments: Vec<Alignment>,
        id: Option<String>,
        classes: Vec<String>,
    },
    TableRow {
        head: bool,
    },
    TableCell,
    /// `number` counts from 1, in the order the writing first points at
    /// each note. `id` is the note's, and `back` its first marker's.
    FootnoteDefinition {
        label: String,
        number: u32,
        id: String,
        back: String,
    },
    Text(String),
    /// Text that reaches the page exactly as typed: an escaped call, a
    /// character typography sets typed as an escape or an entity, and
    /// writing inside raw HTML's code, `<pre>`, `<style>`, `<script>` or a
    /// comment.
    Literal(String),
    Code(String),
    /// Raw HTML: one tag among the words, or one line of an HTML block.
    Html(String),
    /// `text` is the maths as written, without its dollars; `mathml` is
    /// none until the writing's maths is made MathML, and stays none where
    /// it can't be.
    Maths {
        display: bool,
        text: String,
        mathml: Option<String>,
    },
    SoftBreak,
    HardBreak,
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        destination: String,
        title: String,
    },
    /// Its children are its description.
    Image {
        destination: String,
        title: String,
    },
    /// `repeat` counts the markers pointing at one note, from 1. `id` is
    /// the marker's, and `note` the id of the note it points at.
    FootnoteReference {
        label: String,
        number: u32,
        repeat: u32,
        id: String,
        note: String,
    },
    /// A task list's checkbox.
    Task {
        done: bool,
    },
    /// A call standing alone in place of a paragraph is a block; anywhere
    /// else it is among the words. A call that wraps writing is always a
    /// block, the writing its children.
    Call(Box<Call>),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl CalloutKind {
    pub(crate) fn word(self) -> &'static str {
        match self {
            CalloutKind::Note => "note",
            CalloutKind::Tip => "tip",
            CalloutKind::Important => "important",
            CalloutKind::Warning => "warning",
            CalloutKind::Caution => "caution",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Alignment {
    None,
    Left,
    Centre,
    Right,
}

impl Node {
    pub(crate) fn new(kind: Kind, place: Place) -> Self {
        Node {
            kind,
            place,
            children: Vec::new(),
        }
    }

    /// Whether the node sits among the words rather than standing as a block.
    pub(crate) fn is_inline(&self) -> bool {
        if let Kind::Call(call) = &self.kind {
            return !call.wraps();
        }
        matches!(
            self.kind,
            Kind::Text(_)
                | Kind::Literal(_)
                | Kind::Code(_)
                | Kind::Html(_)
                | Kind::Maths { .. }
                | Kind::SoftBreak
                | Kind::HardBreak
                | Kind::Emphasis
                | Kind::Strong
                | Kind::Strikethrough
                | Kind::Link { .. }
                | Kind::Image { .. }
                | Kind::FootnoteReference { .. }
                | Kind::Task { .. }
        )
    }

    /// Visits this node and everything under it, in the order written.
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Node)) {
        f(self);
        for child in &self.children {
            child.walk(f);
        }
    }

    /// The same, for a pass that changes the tree.
    pub(crate) fn walk_mut(&mut self, f: &mut impl FnMut(&mut Node)) {
        f(self);
        for child in &mut self.children {
            child.walk_mut(f);
        }
    }
}
