// Changed from oxhttp 0.3.3 for Statac: response decoding, content decoding and request-body reading are removed, HTTPS URLs are refused, and `skip_request_body` reads past a request body.
use crate::serve::server::model::header::{
    CONTENT_ENCODING, CONTENT_LENGTH, HOST, TRANSFER_ENCODING,
};
use crate::serve::server::model::request::Builder as RequestBuilder;
use crate::serve::server::model::uri::{Authority, Scheme};
use crate::serve::server::model::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Uri, Version,
};
use crate::serve::server::utils::invalid_data_error;
use httparse::Header;
use std::cmp::min;
use std::io::{BufRead, Error, ErrorKind, Read, Result, copy, sink};
use std::str::FromStr;

const DEFAULT_SIZE: usize = 1024;
const MAX_HEADER_SIZE: u64 = 8 * 1024;

pub fn decode_request_headers(reader: &mut impl BufRead) -> Result<RequestBuilder> {
    // Let's read the headers
    let buffer = read_header_bytes(reader)?;
    let mut headers = [httparse::EMPTY_HEADER; DEFAULT_SIZE];
    let mut parsed_request = httparse::Request::new(&mut headers);
    if parsed_request
        .parse(&buffer)
        .map_err(invalid_data_error)?
        .is_partial()
    {
        return Err(invalid_data_error(
            "Partial HTTP headers containing two line jumps",
        ));
    }

    // We build the request
    let mut request = Request::builder();
    decode_headers(parsed_request.headers, request.headers_mut().unwrap())?;
    if let Some(version) = parsed_request.version {
        request = request.version(match version {
            0 => Version::HTTP_10,
            1 => Version::HTTP_11,
            _ => {
                return Err(invalid_data_error(format!(
                    "Unsupported HTTP version {version}"
                )));
            }
        });
    }

    // Method
    request = request.method(
        Method::from_str(
            parsed_request
                .method
                .ok_or_else(|| invalid_data_error("No method in the HTTP request"))?,
        )
        .map_err(invalid_data_error)?,
    );

    // URI
    let path = parsed_request
        .path
        .ok_or_else(|| invalid_data_error("No path in the HTTP request"))?;
    let mut uri_parts = Uri::try_from(path)
        .map_err(invalid_data_error)?
        .into_parts();
    if *uri_parts.scheme.get_or_insert(Scheme::HTTP) != Scheme::HTTP {
        return Err(invalid_data_error("The HTTP URL scheme must be 'http"));
    }
    if uri_parts.authority.is_none() {
        uri_parts.authority = Some(
            Authority::try_from(
                request
                    .headers_ref()
                    .and_then(|headers| unique_header(headers, &HOST).transpose())
                    .ok_or_else(|| invalid_data_error("No host header in HTTP request"))??
                    .as_bytes(),
            )
            .map_err(|e| invalid_data_error(format!("Invalid host header value: {e}")))?,
        );
    }
    Ok(request.uri(Uri::from_parts(uri_parts).unwrap()))
}

/// Reads past the request's body, which nothing answering reads, leaving
/// `reader` where the connection's next request begins.
pub fn skip_request_body(
    request: RequestBuilder,
    reader: &mut impl BufRead,
) -> Result<Request<()>> {
    if let Some(headers) = request.headers_ref() {
        skip_body(headers, reader)?;
    }
    request
        .body(())
        .map_err(|e| invalid_data_error(format!("Unexpected error when parsing the request: {e}")))
}

fn read_header_bytes(reader: impl BufRead) -> Result<Vec<u8>> {
    let mut reader = reader.take(2 * MAX_HEADER_SIZE); // Makes sure we do not buffer too much
    let mut buffer = Vec::with_capacity(DEFAULT_SIZE);
    loop {
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            return Err(Error::new(
                ErrorKind::ConnectionAborted,
                if buffer.is_empty() {
                    "Empty HTTP request"
                } else {
                    "Interrupted HTTP request"
                },
            ));
        }
        if buffer.len() > (MAX_HEADER_SIZE as usize) {
            return Err(invalid_data_error("The headers size must fit in 8kb"));
        }
        // We normalize line ends to plain \n
        if buffer.ends_with(b"\r\n") {
            buffer.pop();
            buffer.pop();
            buffer.push(b'\n')
        }
        if buffer.ends_with(b"\n\n") {
            break; // end of buffer
        }
    }
    Ok(buffer)
}

fn skip_body(headers: &HeaderMap, reader: &mut impl BufRead) -> Result<()> {
    let content_length = unique_header(headers, &CONTENT_LENGTH)?;
    let transfer_encoding = unique_header(headers, &TRANSFER_ENCODING)?;
    unique_header(headers, &CONTENT_ENCODING)?;
    match (content_length, transfer_encoding) {
        (Some(_), Some(_)) => Err(invalid_data_error(
            "Transfer-Encoding and Content-Length should not be set at the same time",
        )),
        (Some(content_length), None) => {
            let len = content_length
                .to_str()
                .map_err(invalid_data_error)?
                .parse::<u64>()
                .map_err(invalid_data_error)?;
            let skipped = copy(&mut reader.take(len), &mut sink())?;
            if skipped < len {
                return Err(Error::new(
                    ErrorKind::ConnectionAborted,
                    format!(
                        "The body was expected to contain {len} bytes but we have been able to only read {skipped}"
                    ),
                ));
            }
            Ok(())
        }
        (None, Some(transfer_encoding)) => {
            if transfer_encoding.as_ref().eq_ignore_ascii_case(b"chunked") {
                let mut chunks = ChunkedDecoder {
                    reader,
                    buffer: Vec::with_capacity(DEFAULT_SIZE),
                    is_start: true,
                    chunk_position: 0,
                    chunk_size: 0,
                    trailers: None,
                };
                copy(&mut chunks, &mut sink()).map(drop)
            } else {
                Err(invalid_data_error(format!(
                    "Transfer-Encoding: {} is not supported",
                    transfer_encoding.to_str().map_err(invalid_data_error)?
                )))
            }
        }
        (None, None) => Ok(()),
    }
}

fn decode_headers(from: &[Header<'_>], to: &mut HeaderMap) -> Result<()> {
    for header in from {
        to.try_append(
            HeaderName::try_from(header.name)
                .map_err(|e| invalid_data_error(format!("Invalid header name: {e}")))?,
            HeaderValue::try_from(header.value)
                .map_err(|e| invalid_data_error(format!("Invalid header value: {e}")))?,
        )
        .map_err(|e| invalid_data_error(format!("Too many headers: {e}")))?;
    }
    Ok(())
}

fn unique_header<'a>(headers: &'a HeaderMap, name: &HeaderName) -> Result<Option<&'a HeaderValue>> {
    let mut headers = headers.get_all(name).iter();
    let value = headers.next();
    if headers.next().is_some() {
        return Err(invalid_data_error(format!("{name} must be set only once")));
    }
    Ok(value)
}

struct ChunkedDecoder<R: BufRead> {
    reader: R,
    buffer: Vec<u8>,
    is_start: bool,
    chunk_position: usize,
    chunk_size: usize,
    trailers: Option<HeaderMap>,
}

impl<R: BufRead> Read for ChunkedDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            // In case we still have data
            if self.chunk_position < self.chunk_size {
                let inner_buf = self.reader.fill_buf()?;
                if inner_buf.is_empty() {
                    return Err(invalid_data_error(
                        "Unexpected stream end in the middle of a chunked content",
                    ));
                }
                let size = min(
                    min(buf.len(), inner_buf.len()),
                    self.chunk_size - self.chunk_position,
                );
                buf[..size].copy_from_slice(&inner_buf[..size]);
                self.reader.consume(size);
                self.chunk_position += size;
                return Ok(size);
            }

            if self.is_start {
                self.is_start = false;
            } else if self.trailers.is_some() {
                return Ok(0); // We already read the trailers, it means we have finished reading
            } else {
                // chunk end
                self.buffer.clear();
                (&mut self.reader)
                    .take(2)
                    .read_until(b'\n', &mut self.buffer)?;
                if self.buffer != b"\r\n" && self.buffer != b"\n" {
                    return Err(invalid_data_error("Invalid chunked element end"));
                }
            }

            // We load a new chunk
            self.buffer.clear();
            (&mut self.reader)
                .take(2 * MAX_HEADER_SIZE)
                .read_until(b'\n', &mut self.buffer)?;
            if self.buffer.len() > (MAX_HEADER_SIZE as usize) {
                return Err(invalid_data_error("The chunk size size must fit in 8kb"));
            }
            self.chunk_position = 0;
            let Ok(httparse::Status::Complete((read, chunk_size))) =
                httparse::parse_chunk_size(&self.buffer)
            else {
                return Err(invalid_data_error("Invalid chunked header"));
            };
            if read != self.buffer.len() {
                return Err(invalid_data_error("Chunked header containing a line jump"));
            }
            self.chunk_size = chunk_size.try_into().map_err(invalid_data_error)?;

            if self.chunk_size == 0 {
                // we read the trailers
                let mut reader = (&mut self.reader).take(2 * MAX_HEADER_SIZE); // Makes sure we do not buffer too much
                self.buffer.clear();
                self.buffer.push(b'\n');
                loop {
                    if reader.read_until(b'\n', &mut self.buffer)? == 0 {
                        return Err(invalid_data_error("Missing chunked encoding end"));
                    }
                    if self.buffer.len() > (MAX_HEADER_SIZE as usize) {
                        return Err(invalid_data_error("The trailers size must fit in 8kb"));
                    }

                    if self.buffer.ends_with(b"\r\n") {
                        self.buffer.pop();
                        self.buffer.pop();
                        self.buffer.push(b'\n')
                    }
                    if self.buffer.ends_with(b"\n\n") {
                        break; // end of buffer
                    }
                }
                let mut trailers = [httparse::EMPTY_HEADER; DEFAULT_SIZE];
                let httparse::Status::Complete((read, parsed_trailers)) =
                    httparse::parse_headers(&self.buffer[1..], &mut trailers)
                        .map_err(invalid_data_error)?
                else {
                    return Err(invalid_data_error(
                        "Partial HTTP headers containing two line jumps",
                    ));
                };
                if read != self.buffer.len() - 1 {
                    return Err(invalid_data_error(
                        "Invalid data at the end of the trailer section",
                    ));
                }
                let mut trailers = HeaderMap::new();
                decode_headers(parsed_trailers, &mut trailers)?;
                self.trailers = Some(trailers);
                return Ok(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::server::model::HeaderName;
    use std::io::{Cursor, read_to_string};

    #[test]
    fn decode_request_target_origin_form() -> Result<()> {
        let request = decode_request_headers(
            &mut b"GET /where?q=now HTTP/1.1\nHost: www.example.org\n\n".as_slice(),
        )?
        .body(())
        .unwrap();
        assert_eq!(
            request.uri().to_string(),
            "http://www.example.org/where?q=now"
        );
        Ok(())
    }

    #[test]
    fn decode_request_target_absolute_form_with_host() -> Result<()> {
        let request = decode_request_headers(
            &mut
              b"GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1\nHost: example.com\n\n".as_slice()
            ,
        )?.body(()).unwrap();
        assert_eq!(
            request.uri().to_string(),
            "http://www.example.org/pub/WWW/TheProject.html"
        );
        Ok(())
    }

    #[test]
    fn decode_request_target_absolute_form_without_host() -> Result<()> {
        let request = decode_request_headers(
            &mut b"GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1\n\n".as_slice(),
        )?
        .body(())
        .unwrap();
        assert_eq!(
            request.uri().to_string(),
            "http://www.example.org/pub/WWW/TheProject.html"
        );
        Ok(())
    }

    #[test]
    fn decode_request_target_relative_form_without_host() {
        assert!(
            decode_request_headers(&mut b"GET /pub/WWW/TheProject.html HTTP/1.1\n\n".as_slice(),)
                .is_err()
        );
    }

    #[test]
    fn decode_request_target_absolute_form_wrong_scheme() {
        assert!(
            decode_request_headers(
                &mut b"GET https://www.example.org/pub/WWW/TheProject.html HTTP/1.1\n\n".as_slice(),
            )
            .is_err()
        );
    }

    #[test]
    fn decode_invalid_request_target_relative_form_with_host() {
        assert!(
            decode_request_headers(
                &mut b"GET /foo<bar HTTP/1.1\nhost: www.example.com\n\n".as_slice(),
            )
            .is_err()
        );
    }

    #[test]
    fn decode_request_target_asterisk_form() -> Result<()> {
        let request = decode_request_headers(
            &mut b"OPTIONS * HTTP/1.1\nHost: www.example.org:8001\n\n".as_slice(),
        )?
        .body(())
        .unwrap();
        assert_eq!(request.uri().path(), "*");
        Ok(())
    }

    #[test]
    fn decode_request_with_header() -> Result<()> {
        let request = decode_request_headers(
            &mut b"GET / HTTP/1.1\nHost: www.example.org:8001\nFoo: v1\nbar: vbar\nfoo: v2\n\n"
                .as_slice(),
        )?
        .body(())
        .unwrap();
        assert_eq!(request.uri().to_string(), "http://www.example.org:8001/");
        assert_eq!(
            request
                .headers()
                .get_all(HeaderName::from_str("foo").unwrap())
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["v1", "v2"]
        );
        assert_eq!(
            request
                .headers()
                .get(HeaderName::from_str("Bar").unwrap())
                .unwrap(),
            "vbar"
        );
        Ok(())
    }

    #[test]
    fn skip_request_with_body() -> Result<()> {
        let mut read =
            b"GET / HTTP/1.1\nHost: www.example.org:8001\ncontent-length: 9\n\nfoobarbarnext"
                .as_slice();
        skip_request_body(decode_request_headers(&mut read)?, &mut read)?;
        assert_eq!(read_to_string(read)?, "next");
        Ok(())
    }

    #[test]
    fn decode_request_empty_header_name() {
        assert!(
            decode_request_headers(
                &mut b"GET / HTTP/1.1\nHost: www.example.org:8001\n: foo".as_slice()
            )
            .is_err()
        );
    }

    #[test]
    fn decode_request_invalid_header_name_char() {
        assert!(
            decode_request_headers(
                &mut b"GET / HTTP/1.1\nHost: www.example.org:8001\nCont\xE9: foo".as_slice()
            )
            .is_err()
        );
    }

    #[test]
    fn decode_request_invalid_header_value_char() {
        assert!(
            decode_request_headers(
                &mut b"GET / HTTP/1.1\nHost: www.example.org:8001\nCont\t: foo\rbar\r\nTest: test"
                    .as_slice()
            )
            .is_err()
        );
    }

    #[test]
    fn decode_request_empty() {
        assert_eq!(
            decode_request_headers(&mut b"".as_slice())
                .err()
                .map(|e| e.kind()),
            Some(ErrorKind::ConnectionAborted)
        );
    }

    #[test]
    fn decode_request_stop_in_header() {
        assert_eq!(
            decode_request_headers(&mut b"GET /\r\n".as_slice())
                .err()
                .map(|e| e.kind()),
            Some(ErrorKind::ConnectionAborted)
        );
    }

    #[test]
    fn skip_request_stop_in_body() -> Result<()> {
        let mut read =
            b"POST / HTTP/1.1\r\nhost: example.com\r\ncontent-length: 12\r\n\r\nfoobar".as_slice();
        assert_eq!(
            skip_request_body(decode_request_headers(&mut read)?, &mut read)
                .err()
                .map(|e| e.kind()),
            Some(ErrorKind::ConnectionAborted)
        );
        Ok(())
    }

    #[test]
    fn skip_request_http_1_0() -> Result<()> {
        let mut read =
            b"POST http://example.com/foo HTTP/1.0\r\ncontent-length: 6\r\n\r\nfoobar".as_slice();
        let request = skip_request_body(decode_request_headers(&mut read)?, &mut read)?;
        assert_eq!(request.version(), Version::HTTP_10);
        assert_eq!(request.uri().to_string(), "http://example.com/foo");
        Ok(())
    }

    #[test]
    fn skip_request_unsupported_transfer_encoding() -> Result<()> {
        let mut read = b"POST / HTTP/1.1\r\nhost: example.com\r\ncontent-length: 12\r\ntransfer-encoding: foo\r\n\r\nfoobar".as_slice();
        assert!(skip_request_body(decode_request_headers(&mut read)?, &mut read).is_err());
        Ok(())
    }

    const CHUNKED: &str =
        "POST / HTTP/1.1\r\nhost: example.com\r\ntransfer-encoding:chunked\r\n\r\n";

    /// What is left to read once a chunked body of `payload` is skipped.
    fn after_chunked(payload: &str) -> Result<String> {
        let mut read = Cursor::new(format!("{CHUNKED}{payload}").into_bytes());
        skip_request_body(decode_request_headers(&mut read)?, &mut read)?;
        read_to_string(read)
    }

    #[test]
    fn skip_request_with_chunked_payload() -> Result<()> {
        assert_eq!(
            after_chunked("4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nnext")?,
            "next"
        );
        Ok(())
    }

    #[test]
    fn skip_request_with_trailer() -> Result<()> {
        assert_eq!(
            after_chunked(
                "4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\ntest: foo\r\n\r\nnext",
            )?,
            "next"
        );
        Ok(())
    }

    #[test]
    fn skip_request_with_invalid_chunk_header() {
        assert!(after_chunked("h\r\nWiki\r\n0\r\n\r\n").is_err());
    }

    #[test]
    fn skip_request_with_invalid_trailer() {
        assert!(after_chunked("f\r\nWiki\r\n0\r\ntest\n: foo\r\n\r\n").is_err());
    }

    #[test]
    fn skip_request_with_not_ended_trailer() {
        assert!(after_chunked("f\r\nWiki").is_err());
    }

    #[test]
    fn skip_request_content_length_and_transfer_encoding() {
        let mut read = b"POST / HTTP/1.1\r\nhost: example.com\r\ntransfer-encoding:chunked\r\ncontent-length: 222\r\n\r\n".as_slice();
        assert!(skip_request_body(decode_request_headers(&mut read).unwrap(), &mut read).is_err());
    }
}
