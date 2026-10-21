//! `!!binary`: bytes written in base64 (RFC 4648's alphabet), with any
//! whitespace inside the value ignored, as a block scalar spreads it over
//! lines.

/// The bytes `text` spells, or none where it isn't base64.
pub(super) fn decode(text: &str) -> Option<Vec<u8>> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .collect();
    let padding = digits.iter().rev().take_while(|&&b| b == b'=').count();
    let body = &digits[..digits.len() - padding];
    // Padding fills the last group of four, and only that.
    if padding > 2 || (padding > 0 && !digits.len().is_multiple_of(4)) || body.len() % 4 == 1 {
        return None;
    }
    let mut bytes = Vec::with_capacity(body.len() * 3 / 4);
    let mut bits: u32 = 0;
    let mut held = 0;
    for &digit in body {
        bits = (bits << 6) | u32::from(value(digit)?);
        held += 6;
        if held >= 8 {
            held -= 8;
            bytes.push((bits >> held) as u8);
            bits &= (1 << held) - 1;
        }
    }
    // The bits left over pad the last byte, and are always 0.
    if bits != 0 {
        return None;
    }
    Some(bytes)
}

fn value(digit: u8) -> Option<u8> {
    match digit {
        b'A'..=b'Z' => Some(digit - b'A'),
        b'a'..=b'z' => Some(digit - b'a' + 26),
        b'0'..=b'9' => Some(digit - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_read_with_whitespace_ignored() {
        let cases: [(&str, Option<&[u8]>); 12] = [
            ("", Some(b"")),
            ("SGVsbG8=", Some(b"Hello")),
            ("SGVsbG8", Some(b"Hello")),
            ("SGVs\n  bG8h", Some(b"Hell\x6f!")),
            ("TWFu", Some(b"Man")),
            ("TWE=", Some(b"Ma")),
            ("TQ==", Some(b"M")),
            ("/w==", Some(b"\xff")),
            ("SGVsbG8*", None),
            ("SGVsbG8===", None),
            ("SG=sbG8=", None),
            ("S", None),
        ];
        for (written, bytes) in cases {
            assert_eq!(decode(written).as_deref(), bytes, "{written:?}");
        }
        // Bits a last group can't use must be 0, so one spelling is one value.
        assert_eq!(decode("TR=="), None);
    }
}
