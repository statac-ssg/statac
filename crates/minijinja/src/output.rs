// Changed from MiniJinja 3.0.1 for Statac: a held-back body may be written into a writer the environment supplies, which makes its value.
use std::ptr::addr_of_mut;
use std::{fmt, io};

use crate::environment::Environment;
use crate::error::{Error, ErrorKind};
use crate::utils::AutoEscape;
use crate::value::Value;

/// How should output be captured?
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "unstable_machinery_serde", derive(serde::Serialize))]
pub enum CaptureMode {
    Capture,
    #[cfg(feature = "multi_template")]
    Discard,
}

/// A writer a held-back body is written into in place of a string.
///
/// A held-back body is template output the engine renders before it is
/// written, if at all: the body of a `{% set %}` block, a `{% filter %}`
/// block, a macro (and so of a `{% call %}` block), a block rendered by
/// `super()`, and a recursive loop's level used as a value.  The writer is
/// made by the function given to
/// [`set_held`](crate::Environment::set_held).
pub trait Held: fmt::Write {
    /// Gives the body's value, made from its text: marked safe where
    /// `safe`.
    fn finish(self: Box<Self>, safe: bool) -> Value;
}

/// Where a held-back body is written.
pub(crate) enum Capture {
    Text(String),
    Held(Box<dyn Held>),
}

impl Capture {
    /// Creates the writer `env` gives a held-back body.
    pub(crate) fn new(env: &Environment) -> Self {
        match env.held() {
            Some(held) => Capture::Held(held),
            None => Capture::Text(String::new()),
        }
    }

    /// Gives the value made of the text written: marked safe where `safe`.
    pub(crate) fn finish(self, safe: bool) -> Value {
        match self {
            Capture::Text(text) if safe => Value::from_safe_string(text),
            Capture::Text(text) => Value::from(text),
            Capture::Held(held) => held.finish(safe),
        }
    }
}

impl fmt::Write for Capture {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Capture::Text(text) => text.write_str(s),
            Capture::Held(held) => held.write_str(s),
        }
    }
}

/// An abstraction over [`fmt::Write`](std::fmt::Write) for the rendering.
///
/// This is a utility type used in the engine which can be written into like one
/// can write into an [`std::fmt::Write`] value.  It's primarily used internally
/// in the engine but it's also passed to the custom formatter function.
pub struct Output<'a> {
    w: *mut (dyn fmt::Write + 'a),
    target: *mut (dyn fmt::Write + 'a),
    capture_stack: Vec<Option<Capture>>,
}

impl<'a> Output<'a> {
    /// Creates a new output.
    pub(crate) fn new(w: &'a mut (dyn fmt::Write + 'a)) -> Self {
        Self {
            w,
            target: w,
            capture_stack: Vec::new(),
        }
    }

    /// Creates a null output that writes nowhere.
    pub(crate) fn null() -> Self {
        // The null writer also has a single entry on the discarding capture
        // stack.  In fact, `w` is more or less useless here as we always
        // shadow it.  This is done so that `is_discarding` returns true.
        Self {
            w: NullWriter::get_mut(),
            target: NullWriter::get_mut(),
            capture_stack: vec![None],
        }
    }

    /// Begins capturing into a string, or the writer `env` gives, or discard.
    pub(crate) fn begin_capture(&mut self, mode: CaptureMode, env: &Environment) {
        self.capture_stack.push(match mode {
            CaptureMode::Capture => Some(Capture::new(env)),
            #[cfg(feature = "multi_template")]
            CaptureMode::Discard => None,
        });
        self.retarget();
    }

    /// Ends capturing and returns the captured string as value.
    pub(crate) fn end_capture(&mut self, auto_escape: &AutoEscape) -> Value {
        let rv = if let Some(captured) = self.capture_stack.pop().unwrap() {
            captured.finish(!matches!(auto_escape, AutoEscape::None))
        } else {
            Value::UNDEFINED
        };
        self.retarget();
        rv
    }

    fn retarget(&mut self) {
        self.target = match self.capture_stack.last_mut() {
            Some(Some(stream)) => stream,
            Some(None) => NullWriter::get_mut(),
            None => self.w,
        };
    }

    #[inline(always)]
    fn target(&mut self) -> &mut dyn fmt::Write {
        // SAFETY: this is safe because we carefully maintain the capture stack
        // to update self.target whenever it's modified
        unsafe { &mut *self.target }
    }

    /// Returns `true` if the output is discarding.
    #[cfg(feature = "multi_template")]
    #[inline(always)]
    pub(crate) fn is_discarding(&self) -> bool {
        matches!(self.capture_stack.last(), Some(None))
    }

    /// Writes some data to the underlying buffer contained within this output.
    #[inline]
    pub fn write_str(&mut self, s: &str) -> fmt::Result {
        self.target().write_str(s)
    }

    /// Writes some formatted information into this instance.
    #[inline]
    pub fn write_fmt(&mut self, a: fmt::Arguments<'_>) -> fmt::Result {
        self.target().write_fmt(a)
    }
}

impl fmt::Write for Output<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        fmt::Write::write_str(self.target(), s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        fmt::Write::write_char(self.target(), c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        fmt::Write::write_fmt(self.target(), args)
    }
}

pub struct NullWriter;

impl NullWriter {
    /// Returns a reference to the null writer.
    pub fn get_mut() -> &'static mut NullWriter {
        static mut NULL_WRITER: NullWriter = NullWriter;
        // SAFETY: this is safe as the null writer is a ZST
        unsafe { &mut *addr_of_mut!(NULL_WRITER) }
    }
}

impl fmt::Write for NullWriter {
    #[inline]
    fn write_str(&mut self, _s: &str) -> fmt::Result {
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, _c: char) -> fmt::Result {
        Ok(())
    }
}

pub struct WriteWrapper<W> {
    pub w: W,
    pub err: Option<io::Error>,
}

impl<W> WriteWrapper<W> {
    /// Replaces the given error with the held error if available.
    pub fn take_err(&mut self, original: Error) -> Error {
        self.err
            .take()
            .map(|io_err| {
                Error::new(ErrorKind::WriteFailure, "I/O error during rendering")
                    .with_source(io_err)
            })
            .unwrap_or(original)
    }
}

impl<W: io::Write> fmt::Write for WriteWrapper<W> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.w.write_all(s.as_bytes()).map_err(|e| {
            self.err = Some(e);
            fmt::Error
        })
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.w
            .write_all(c.encode_utf8(&mut [0; 4]).as_bytes())
            .map_err(|e| {
                self.err = Some(e);
                fmt::Error
            })
    }
}

#[cfg(all(test, feature = "macros"))]
mod tests {
    use std::fmt;
    use std::sync::{Arc, Mutex};

    use super::Held;
    use crate::value::Value;
    use crate::Environment;

    /// Notes each body it is written and gives it back in capitals.
    struct Shouting {
        text: String,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl fmt::Write for Shouting {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.text.push_str(s);
            Ok(())
        }
    }

    impl Held for Shouting {
        fn finish(self: Box<Self>, safe: bool) -> Value {
            self.seen.lock().unwrap().push(self.text.clone());
            let text = self.text.to_uppercase();
            if safe {
                Value::from_safe_string(text)
            } else {
                Value::from(text)
            }
        }
    }

    #[test]
    fn test_held_writes_set_blocks_and_macro_bodies() {
        let source = "{% macro m(a) %}<p>{{ a }}</p>{% endmacro %}\
                      {% set s %}<i>{{ m('x') }}</i>{% endset %}[{{ s }}]";
        let env = Environment::new();
        assert_eq!(env.render_str(source, ()).unwrap(), "[<i><p>x</p></i>]");

        let mut env = Environment::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let held = seen.clone();
        env.set_held(move || {
            Box::new(Shouting {
                text: String::new(),
                seen: held.clone(),
            })
        });
        assert_eq!(env.render_str(source, ()).unwrap(), "[<I><P>X</P></I>]");
        assert_eq!(*seen.lock().unwrap(), ["<p>x</p>", "<i><P>X</P></i>"]);
    }
}
