//! The YAML 1.2.2 core schema's reading of a plain scalar (spec 10.3.2).

/// What the core schema makes of a plain scalar.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Resolved {
    Null,
    Bool(bool),
    /// None where the integer is too large for an `i128`.
    Int(Option<i128>),
    Float(f64),
    Str,
}

/// How YAML's standard reading resolves a plain scalar written as `text`.
/// Quoted and block scalars are always strings and never come here.
pub fn resolve(text: &str) -> Resolved {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Resolved::Null,
        "true" | "True" | "TRUE" => return Resolved::Bool(true),
        "false" | "False" | "FALSE" => return Resolved::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
            return Resolved::Float(f64::INFINITY);
        }
        "-.inf" | "-.Inf" | "-.INF" => return Resolved::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Resolved::Float(f64::NAN),
        _ => {}
    }
    if let Some(digits) = text.strip_prefix("0o")
        && !digits.is_empty()
        && digits.bytes().all(|b| (b'0'..=b'7').contains(&b))
    {
        return Resolved::Int(i128::from_str_radix(digits, 8).ok());
    }
    if let Some(digits) = text.strip_prefix("0x")
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Resolved::Int(i128::from_str_radix(digits, 16).ok());
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    if !unsigned.is_empty() && unsigned.bytes().all(|b| b.is_ascii_digit()) {
        return Resolved::Int(text.parse().ok());
    }
    if is_float(unsigned) {
        // The core grammar is a subset of what Rust's float parser takes.
        if let Ok(value) = text.parse() {
            return Resolved::Float(value);
        }
    }
    Resolved::Str
}

/// `( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?`, sign removed.
fn is_float(text: &str) -> bool {
    let b = text.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let from = *i;
        while b.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        *i - from
    };
    if b.first() == Some(&b'.') {
        i = 1;
        if digits(&mut i) == 0 {
            return false;
        }
    } else {
        if digits(&mut i) == 0 {
            return false;
        }
        if b.get(i) == Some(&b'.') {
            i += 1;
            digits(&mut i);
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'-' | b'+')) {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    i == b.len()
}

/// Whether reading `text` as `resolved` keeps its characters: whether the
/// value written back in its plainest form is `text` again (`1984`, `2.5`,
/// `true`, but not `3.10`, `010` or `True`).
pub fn keeps_characters(text: &str, resolved: Resolved) -> bool {
    match resolved {
        Resolved::Null | Resolved::Str => false,
        Resolved::Bool(value) => text == if value { "true" } else { "false" },
        Resolved::Int(_) => {
            let digits = text.strip_prefix('-').unwrap_or(text);
            text != "-0"
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'))
        }
        Resolved::Float(value) => {
            value.is_finite() && text.contains('.') && value.to_string() == text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Resolved::*;

    #[test]
    fn plain_scalars_resolve_by_the_core_schema() {
        let cases = [
            ("", Null),
            ("~", Null),
            ("null", Null),
            ("Null", Null),
            ("NULL", Null),
            ("nULL", Str),
            ("true", Bool(true)),
            ("True", Bool(true)),
            ("FALSE", Bool(false)),
            ("tRUE", Str),
            // Booleans to YAML 1.1, text to 1.2.
            ("yes", Str),
            ("no", Str),
            ("on", Str),
            ("off", Str),
            ("y", Str),
            ("0", Int(Some(0))),
            ("-0", Int(Some(0))),
            ("+12", Int(Some(12))),
            ("-3", Int(Some(-3))),
            // Decimal in 1.2, octal in 1.1.
            ("010", Int(Some(10))),
            ("0o10", Int(Some(8))),
            ("0o8", Str),
            ("0x10", Int(Some(16))),
            ("0x1F", Int(Some(31))),
            ("0xg", Str),
            ("0b101", Str),
            ("1_000", Str),
            ("1:30", Str),
            ("3.10", Float(3.1)),
            ("1e3", Float(1000.0)),
            ("1E+3", Float(1000.0)),
            ("1e", Str),
            (".5", Float(0.5)),
            ("5.", Float(5.0)),
            ("+.5", Float(0.5)),
            (".inf", Float(f64::INFINITY)),
            ("-.Inf", Float(f64::NEG_INFINITY)),
            ("nan", Str),
            ("1.2.3", Str),
            ("12abc", Str),
            ("2026-10-04", Str),
            ("Post 1", Str),
            ("170141183460469231731687303715884105728", Int(None)),
        ];
        for (text, expected) in cases {
            assert_eq!(resolve(text), expected, "{text:?}");
        }
        assert!(matches!(resolve(".NaN"), Float(f) if f.is_nan()));
    }

    #[test]
    fn some_readings_keep_their_characters() {
        let cases = [
            ("1984", true),
            ("-7", true),
            ("0", true),
            ("2.5", true),
            ("true", true),
            ("3.10", false),
            ("1e3", false),
            ("0x10", false),
            ("0o10", false),
            ("010", false),
            ("+12", false),
            ("-0", false),
            ("1.0", false),
            (".5", false),
            (".inf", false),
            ("True", false),
            ("~", false),
            ("null", false),
        ];
        for (text, kept) in cases {
            assert_eq!(keeps_characters(text, resolve(text)), kept, "{text:?}");
        }
    }
}
