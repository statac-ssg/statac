// Changed from oxhttp 0.3.3 for Statac: request encoding and chunked bodies with trailers are removed; a response body always has a known length.
use crate::serve::server::model::header::{
    ACCEPT_CHARSET, ACCEPT_ENCODING, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_REQUEST_HEADERS,
    CONNECTION, CONTENT_LENGTH, DATE, EXPECT, HOST, ORIGIN, TE, TRAILER, TRANSFER_ENCODING,
    UPGRADE, VIA,
};
use crate::serve::server::model::{Body, HeaderMap, HeaderName, Response, StatusCode, Version};
use crate::serve::server::utils::invalid_input_error;
use std::io::{Result, Write, copy};

pub(crate) fn encode_response<W: Write>(response: &mut Response<Body>, mut writer: W) -> Result<W> {
    let status = response.status();
    let version_str = serialize_version(response.version())?;
    write!(&mut writer, "{version_str} {status}\r\n")?;
    encode_headers(response.headers(), &mut writer)?;
    let must_include_body = does_response_must_include_body(response.status());
    encode_body(response.body_mut(), &mut writer, must_include_body)?;
    Ok(writer)
}

fn encode_headers(headers: &HeaderMap, writer: &mut impl Write) -> Result<()> {
    for (name, value) in headers {
        if !is_forbidden_name(name) {
            write!(writer, "{name}: ")?;
            writer.write_all(value.as_bytes())?;
            write!(writer, "\r\n")?;
        }
    }
    Ok(())
}

fn encode_body(body: &mut Body, writer: &mut impl Write, must_include_body: bool) -> Result<()> {
    let length = body.len();
    if must_include_body || length > 0 {
        write!(writer, "content-length: {length}\r\n\r\n")?;
        copy(body, writer)?;
    } else {
        write!(writer, "\r\n")?;
    }
    Ok(())
}

/// Checks if it is a [forbidden header name](https://fetch.spec.whatwg.org/#forbidden-header-name)
///
/// We removed some of them not managed by this library (`Access-Control-Request-Headers`, `Access-Control-Request-Method`, `DNT`, `Cookie`, `Cookie2`, `Referer`, `Proxy-`, `Sec-`, `Via`...)
fn is_forbidden_name(header: &HeaderName) -> bool {
    header == ACCEPT_CHARSET
        || header == ACCEPT_ENCODING
        || header == ACCESS_CONTROL_REQUEST_HEADERS
        || header == ACCESS_CONTROL_ALLOW_METHODS
        || header == CONNECTION
        || header == CONTENT_LENGTH
        || header == DATE
        || header == EXPECT
        || header == HOST
        || header.as_str() == "keep-alive"
        || header == ORIGIN
        || header == TE
        || header == TRAILER
        || header == TRANSFER_ENCODING
        || header == UPGRADE
        || header == VIA
}

fn does_response_must_include_body(status: StatusCode) -> bool {
    !(status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED)
}

fn serialize_version(version: Version) -> Result<&'static str> {
    match version {
        Version::HTTP_10 => Ok("HTTP/1.0"),
        Version::HTTP_11 => Ok("HTTP/1.1"),
        _ => Err(invalid_input_error(
            "HTTP version {version:?} is not supported",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::server::model::header::ACCEPT;
    use std::str;

    #[test]
    fn encode_response_ok() -> Result<()> {
        let mut response = Response::builder()
            .header(ACCEPT, "application/json")
            .body(Body::from("test test2"))
            .unwrap();
        let buffer = encode_response(&mut response, Vec::new())?;
        assert_eq!(
            str::from_utf8(&buffer).unwrap(),
            "HTTP/1.1 200 OK\r\naccept: application/json\r\ncontent-length: 10\r\n\r\ntest test2"
        );
        Ok(())
    }

    #[test]
    fn encode_response_not_found() -> Result<()> {
        let mut response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
        let buffer = encode_response(&mut response, Vec::new())?;
        assert_eq!(
            str::from_utf8(&buffer).unwrap(),
            "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n"
        );
        Ok(())
    }

    #[test]
    fn encode_response_custom_code() -> Result<()> {
        let mut response = Response::builder().status(499).body(Body::empty()).unwrap();
        let buffer = encode_response(&mut response, Vec::new())?;
        assert_eq!(
            str::from_utf8(&buffer).unwrap(),
            "HTTP/1.1 499 <unknown status code>\r\ncontent-length: 0\r\n\r\n"
        );
        Ok(())
    }

    #[test]
    fn http_2_not_serializable() {
        assert!(
            encode_response(
                &mut Response::builder()
                    .version(Version::HTTP_2)
                    .body(Body::empty())
                    .unwrap(),
                &mut Vec::new()
            )
            .is_err()
        );
    }
}
