//! Addresses written in the site's files, settled to the files in
//! `assets/` they name, and written again to name the published files.

/// The file in `assets/` an address in the stylesheet `from` names, as a
/// browser would settle it, or `None` for one elsewhere: on another site,
/// in `data:`, above `assets/`, or with a backslash a browser and a
/// stylesheet might read differently. `base` is the site's own path.
pub(super) fn settled(from: &str, written: &str, base: &str) -> Option<String> {
    if written.starts_with("//") || written.starts_with('#') || has_scheme(written) {
        return None;
    }
    if written.contains('\\') {
        return None;
    }
    let path = &written[..written.find(['?', '#']).unwrap_or(written.len())];
    if path.is_empty() || path.ends_with('/') {
        return None;
    }
    let path = percent_decoded(path)?;
    let mut pieces: Vec<&str> = Vec::new();
    let rest = match path.strip_prefix('/') {
        Some(rooted) => {
            let base = base.trim_matches('/');
            match base.is_empty() {
                true => rooted,
                false => rooted.strip_prefix(base)?.strip_prefix('/')?,
            }
        }
        None => {
            if let Some((folder, _)) = from.rsplit_once('/') {
                pieces.extend(folder.split('/'));
            }
            &path
        }
    };
    for piece in rest.split('/') {
        match piece {
            "." | "" => {}
            ".." => {
                pieces.pop()?;
            }
            piece => pieces.push(piece),
        }
    }
    Some(pieces.join("/"))
}

/// `written` naming `published` instead: only the file's name changes, so
/// the folders and anything after a `?` or `#` stay as the author wrote
/// them.
pub(super) fn respelled(written: &str, published: &str) -> String {
    let end = written.find(['?', '#']).unwrap_or(written.len());
    let (path, rest) = written.split_at(end);
    let folders = path.rfind('/').map_or("", |slash| &path[..=slash]);
    let file = published.rsplit('/').next().unwrap_or(published);
    format!("{folders}{}{rest}", percent_encoded(file))
}

fn has_scheme(written: &str) -> bool {
    let Some(colon) = written.find(':') else {
        return false;
    };
    let scheme = &written[..colon];
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
}

fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |i: usize| bytes.get(i).and_then(|b| (*b as char).to_digit(16));
        match (bytes[at], hex(at + 1), hex(at + 2)) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                at += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// A file name made safe to write in an address in a stylesheet, quoted
/// or not.
fn percent_encoded(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_control() || " \"'()\\%?#".contains(c) {
            let mut buffer = [0; 4];
            for byte in c.encode_utf8(&mut buffer).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_settled_against_the_stylesheet() {
        let cases = [
            ("parts/site.css", "colours.css", Some("parts/colours.css")),
            ("parts/site.css", "../fonts/a.woff2", Some("fonts/a.woff2")),
            (
                "parts/site.css",
                "./deep/b.png?v=2#here",
                Some("parts/deep/b.png"),
            ),
            ("parts/site.css", "a%20b.css", Some("parts/a b.css")),
            ("parts/site.css", "/top.css", Some("top.css")),
            ("parts/site.css", "/fonts/a.woff2", Some("fonts/a.woff2")),
            ("site.css", "../../above.css", None),
            ("site.css", "../logo.png", None),
            ("site.css", "/logo.png", Some("logo.png")),
            ("site.css", "https://fonts.example/a.css", None),
            ("site.css", "//cdn.example/a.css", None),
            ("site.css", "data:image/png;base64,AAAA", None),
            ("site.css", "#filter", None),
            ("site.css", "sl\\61sh.css", None),
        ];
        for (from, address, expected) in cases {
            assert_eq!(settled(from, address, "").as_deref(), expected, "{address}");
        }
    }

    #[test]
    fn only_the_file_name_is_respelled() {
        assert_eq!(
            respelled("../fonts/a.woff2?v=2#x", "fonts/a.1a2b3c4d.woff2"),
            "../fonts/a.1a2b3c4d.woff2?v=2#x"
        );
        assert_eq!(
            respelled("a%20b.css", "a b.1a2b3c4d.css"),
            "a%20b.1a2b3c4d.css"
        );
        assert_eq!(
            respelled("/top.css", "top.1a2b3c4d.css"),
            "/top.1a2b3c4d.css"
        );
    }

    #[test]
    fn a_rooted_address_carries_the_sites_own_path() {
        assert_eq!(
            settled("site.css", "/blog/a.woff2", "/blog").as_deref(),
            Some("a.woff2")
        );
        assert_eq!(settled("site.css", "/a.woff2", "/blog"), None);
        assert_eq!(settled("site.css", "/blogs/a.woff2", "/blog"), None);
    }
}
