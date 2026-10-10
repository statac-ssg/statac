use crate::serve::server::model::HeaderMap;
use std::borrow::Cow;
use std::fmt;
use std::io::{Cursor, Error, ErrorKind, Read, Result};

/// A request or response [body](https://httpwg.org/http-core/draft-ietf-httpbis-messaging-latest.html#message.body).
///
/// It implements the [`Read`] API.
pub struct Body(BodyAlt);

enum BodyAlt {
    SimpleOwned(Cursor<Vec<u8>>),
    SimpleBorrowed(&'static [u8]),
    Sized {
        content: Box<dyn Read>,
        total_len: u64,
        consumed_len: u64,
    },
    Chunked(Box<dyn ChunkedTransferPayload>),
}

impl Body {
    #[inline]
    pub(crate) fn from_read_and_len(read: impl Read + 'static, len: u64) -> Self {
        Self(BodyAlt::Sized {
            total_len: len,
            consumed_len: 0,
            content: Box::new(read.take(len)),
        })
    }

    /// Creates a [chunked transfer encoding](https://httpwg.org/http-core/draft-ietf-httpbis-messaging-latest.html#chunked.encoding) body with optional [trailers](https://httpwg.org/http-core/draft-ietf-httpbis-semantics-latest.html#trailer.fields).
    #[inline]
    pub fn from_chunked_transfer_payload(payload: impl ChunkedTransferPayload + 'static) -> Self {
        Self(BodyAlt::Chunked(Box::new(payload)))
    }

    /// The empty body
    #[inline]
    pub fn empty() -> Self {
        Self(BodyAlt::SimpleBorrowed(b""))
    }

    /// The number of bytes in the body (if known).
    #[inline]
    pub fn len(&self) -> Option<u64> {
        match &self.0 {
            BodyAlt::SimpleOwned(d) => Some(d.get_ref().len().try_into().unwrap()),
            BodyAlt::SimpleBorrowed(d) => Some(d.len().try_into().unwrap()),
            BodyAlt::Sized { total_len, .. } => Some(*total_len),
            BodyAlt::Chunked(_) => None,
        }
    }

    /// Returns the chunked transfer encoding trailers if they exists and are already received.
    /// You should fully consume the body before attempting to fetch them.
    #[inline]
    pub fn trailers(&self) -> Option<&HeaderMap> {
        match &self.0 {
            BodyAlt::SimpleOwned(_) | BodyAlt::SimpleBorrowed(_) | BodyAlt::Sized { .. } => None,
            BodyAlt::Chunked(c) => c.trailers(),
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
            BodyAlt::Sized { total_len, .. } => s.field("content-length", total_len),
            BodyAlt::Chunked(_) => s.field("transfer-encoding", &"chunked"),
        }
    }
}

impl Read for Body {
    #[inline]
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        match &mut self.0 {
            BodyAlt::SimpleOwned(c) => c.read(buf),
            BodyAlt::SimpleBorrowed(c) => c.read(buf),
            BodyAlt::Sized {
                content,
                consumed_len,
                total_len,
            } => {
                let remaining_size = *total_len - *consumed_len;
                if remaining_size < u64::try_from(buf.len()).unwrap() {
                    buf = &mut buf[..usize::try_from(remaining_size).unwrap()];
                }
                if buf.is_empty() {
                    return Ok(0); // Nothing to read
                }
                let read = content.read(buf)?;
                *consumed_len += u64::try_from(read).unwrap();
                if read == 0 {
                    // We are missing some bytes
                    return Err(Error::new(
                        ErrorKind::ConnectionAborted,
                        format!(
                            "The body was expected to contain {total_len} bytes but we have been able to only read {consumed_len}"
                        ),
                    ));
                }
                Ok(read)
            }
            BodyAlt::Chunked(inner) => inner.read(buf),
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

/// Trait to give to [`Body::from_chunked_transfer_payload`] a body to serialize
/// as [chunked transfer encoding](https://httpwg.org/http-core/draft-ietf-httpbis-messaging-latest.html#chunked.encoding).
///
/// It allows to provide [trailers](https://httpwg.org/http-core/draft-ietf-httpbis-semantics-latest.html#trailer.fields) to serialize.
pub trait ChunkedTransferPayload: Read {
    /// The [trailers](https://httpwg.org/http-core/draft-ietf-httpbis-semantics-latest.html#trailer.fields) to serialize.
    fn trailers(&self) -> Option<&HeaderMap>;
}
