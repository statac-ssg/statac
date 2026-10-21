//! The protocol's framing: each message is a header giving its length in
//! bytes, a blank line, then the message itself as JSON.

use std::io::{self, BufRead, Write};

/// The longest message read. An editor sends a file's whole text in one,
/// which this leaves room for many times over.
const LONGEST: usize = 256 << 20;

/// The next message, or none where the input has ended.
pub(super) fn read(input: &mut dyn BufRead) -> io::Result<Option<String>> {
    let mut length = None;
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            // A blank line ends the headers; one before them is passed over.
            match length {
                Some(_) => break,
                None => continue,
            }
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            let value = value.trim().parse::<usize>();
            length = Some(value.map_err(|_| unreadable("a length that isn't a number"))?);
        }
    }
    let length = length.expect("set before the headers end");
    if length > LONGEST {
        return Err(unreadable("a message too long to read"));
    }
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|_| unreadable("a message that isn't UTF-8"))
}

/// Sends `message`, with its header.
pub(super) fn write(output: &mut dyn Write, message: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{message}", message.len())?;
    output.flush()
}

fn unreadable(what: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}
