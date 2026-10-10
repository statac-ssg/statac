//! Every value carries where it came from, from the moment it is read, so a
//! problem with it never has to reconstruct its blame.

use crate::source::Place;

/// How a value reached the thing it applies to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Arrival {
    /// Written on the thing itself.
    Written,
    /// Written further up and handed down, nearest first; `place` on the
    /// origin is where it was written.
    HandedDown,
    /// Nobody wrote it: Statac's own default.
    Defaulted,
    /// Taken from the name of the page's file, as a date at its start is.
    FileName,
    /// Given on the command line, for the one run.
    CommandLine,
}

/// How a scalar was written. YAML reads the same characters differently by
/// style, and some problems turn on it (`3.10` against `"3.10"`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Style {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
    /// Not a scalar, or not written at all.
    None,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Origin {
    /// None only for a default, or a value given on the command line.
    pub place: Option<Place>,
    pub style: Style,
    pub arrival: Arrival,
}

impl Origin {
    pub fn written(place: Place, style: Style) -> Self {
        Origin {
            place: Some(place),
            style,
            arrival: Arrival::Written,
        }
    }

    pub fn defaulted() -> Self {
        Origin {
            place: None,
            style: Style::None,
            arrival: Arrival::Defaulted,
        }
    }

    pub fn command_line() -> Self {
        Origin {
            place: None,
            style: Style::None,
            arrival: Arrival::CommandLine,
        }
    }

    /// The same written value, as it reaches something further down.
    pub fn handed_down(&self) -> Self {
        Origin {
            arrival: match self.arrival {
                Arrival::Defaulted => Arrival::Defaulted,
                _ => Arrival::HandedDown,
            },
            ..self.clone()
        }
    }
}

/// A value and where it came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sourced<T> {
    pub value: T,
    pub origin: Origin,
}

impl<T> Sourced<T> {
    pub fn new(value: T, origin: Origin) -> Self {
        Sourced { value, origin }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Sourced<U> {
        Sourced {
            value: f(self.value),
            origin: self.origin,
        }
    }

    pub fn as_ref(&self) -> Sourced<&T> {
        Sourced {
            value: &self.value,
            origin: self.origin.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Sources, Span};

    #[test]
    fn handing_down_keeps_the_place_it_was_written() {
        let mut sources = Sources::default();
        let id = sources.add("content/posts/index.md", "layout: post\n".into());
        let place = Place {
            source: id,
            span: Span::new(8, 12),
        };
        let written = Origin::written(place, Style::Plain);
        let below = written.handed_down();
        assert_eq!(below.arrival, Arrival::HandedDown);
        assert_eq!(below.place, Some(place));
        assert_eq!(sources.spelling(below.place.unwrap()), "post");
    }

    #[test]
    fn a_default_handed_down_is_still_a_default() {
        assert_eq!(
            Origin::defaulted().handed_down().arrival,
            Arrival::Defaulted
        );
    }
}
