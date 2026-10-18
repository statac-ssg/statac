// Changed from oxhttp 0.3.3 for Statac: bodies built from a reader, chunked payloads, trailers and gzip or deflate decoding are removed; a body length is always known.
use std::borrow::Cow;
use std::fmt;
use std::io::{Cursor, Read, Result};

/// A response's [body](https://httpwg.org/http-core/draft-ietf-httpbis-messaging-latest.html#message.body).
///
/// It implements the [`Read`] API.
pub struct Body(BodyAlt);

enum BodyAlt {
    SimpleOwned(Cursor<Vec<u8>>),
    SimpleBorrowed(&'static [u8]),
}

impl Body {
    /// The empty body
    #[inline]
    pub fn empty() -> Self {
        Self(BodyAlt::SimpleBorrowed(b""))
    }

    /// The number of bytes in the body.
    #[inline]
    pub fn len(&self) -> u64 {
        match &self.0 {
            BodyAlt::SimpleOwned(d) => d.get_ref().len().try_into().unwrap(),
            BodyAlt::SimpleBorrowed(d) => d.len().try_into().unwrap(),
        }
    }

    /// Reads the full body into a string.
    #[cfg(test)]
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name the tests of the answers read a body by"
    )]
    pub fn to_string(mut self) -> Result<String> {
        let mut buf = String::new();
        self.read_to_string(&mut buf)?;
        Ok(buf)
    }

    fn debug_fields<'a, 'b, 'c>(
        &'b self,
        s: &'c mut fmt::DebugStruct<'b, 'a>,
    ) -> &'c mut fmt::DebugStruct<'b, 'a> {
        match &self.0 {
            BodyAlt::SimpleOwned(d) => s.field("content-length", &d.get_ref().len()),
            BodyAlt::SimpleBorrowed(d) => s.field("content-length", &d.len()),
        }
    }
}

impl Read for Body {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match &mut self.0 {
            BodyAlt::SimpleOwned(c) => c.read(buf),
            BodyAlt::SimpleBorrowed(c) => c.read(buf),
        }
    }
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Vec<u8>> for Body {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self(BodyAlt::SimpleOwned(Cursor::new(data)))
    }
}

impl From<String> for Body {
    #[inline]
    fn from(data: String) -> Self {
        data.into_bytes().into()
    }
}

impl From<&'static [u8]> for Body {
    #[inline]
    fn from(data: &'static [u8]) -> Self {
        Self(BodyAlt::SimpleBorrowed(data))
    }
}

impl From<&'static str> for Body {
    #[inline]
    fn from(data: &'static str) -> Self {
        data.as_bytes().into()
    }
}

impl From<Cow<'static, [u8]>> for Body {
    #[inline]
    fn from(data: Cow<'static, [u8]>) -> Self {
        match data {
            Cow::Borrowed(data) => data.into(),
            Cow::Owned(data) => data.into(),
        }
    }
}

impl From<Cow<'static, str>> for Body {
    #[inline]
    fn from(data: Cow<'static, str>) -> Self {
        match data {
            Cow::Borrowed(data) => data.into(),
            Cow::Owned(data) => data.into(),
        }
    }
}

impl From<()> for Body {
    #[inline]
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl fmt::Debug for Body {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_fields(&mut f.debug_struct("Body")).finish()
    }
}
