//! A release's package read in memory: a gzipped tar file, or on Windows
//! a zip file. Only what the release's packages use is read: files stored
//! or deflated, and in a tar file the long names GNU and POSIX tar write.
//! A link or a folder in a package is passed over. The package's SHA-256
//! has been checked before it is read, so a fault here is a package made
//! wrongly, never one tampered with.

use miniz_oxide::inflate::decompress_to_vec_with_limit;

/// The most bytes a package unpacks to.
const UNPACKED_LIMIT: usize = 1 << 30;

/// The package doesn't read as one.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unreadable;

/// Each file in `package`, a zip file where `zip` says so and otherwise a
/// gzipped tar file, by its path inside with forward slashes.
pub(super) fn files(package: &[u8], zip: bool) -> Result<Vec<(String, Vec<u8>)>, Unreadable> {
    match zip {
        true => zipped(package),
        false => tarred(&gunzipped(package)?),
    }
}

/// What the gzip file `bytes` holds (RFC 1952).
fn gunzipped(bytes: &[u8]) -> Result<Vec<u8>, Unreadable> {
    const EXTRA: u8 = 4;
    const NAME: u8 = 8;
    const COMMENT: u8 = 16;
    const HEADER_CRC: u8 = 2;
    if bytes.get(..3) != Some(&[0x1f, 0x8b, 8]) || bytes.len() < 18 {
        return Err(Unreadable);
    }
    let flags = bytes[3];
    let mut at = 10;
    if flags & EXTRA != 0 {
        at += 2 + usize::from(le16(bytes, at)?);
    }
    for field in [NAME, COMMENT] {
        if flags & field != 0 {
            let rest = bytes.get(at..).ok_or(Unreadable)?;
            at += rest.iter().position(|&byte| byte == 0).ok_or(Unreadable)? + 1;
        }
    }
    if flags & HEADER_CRC != 0 {
        at += 2;
    }
    let deflated = bytes.get(at..bytes.len() - 8).ok_or(Unreadable)?;
    decompress_to_vec_with_limit(deflated, UNPACKED_LIMIT).map_err(|_| Unreadable)
}

/// The files of the tar file `bytes`, in the ustar layout with GNU's and
/// POSIX's long names.
fn tarred(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Unreadable> {
    let mut files = Vec::new();
    let mut at = 0;
    // A name given by the entry before, for the entry after.
    let mut long_name = None;
    while let Some(header) = bytes.get(at..at + 512) {
        if header.iter().all(|&byte| byte == 0) {
            break;
        }
        let size = octal(&header[124..136])?;
        let kind = header[156];
        let body = bytes.get(at + 512..at + 512 + size).ok_or(Unreadable)?;
        at += 512 + size.div_ceil(512) * 512;
        let name = match long_name.take() {
            Some(name) => name,
            None => {
                let name = text(&header[..100]);
                let prefix = text(&header[345..500]);
                match header[257..262] == *b"ustar" && !prefix.is_empty() {
                    true => format!("{prefix}/{name}"),
                    false => name,
                }
            }
        };
        match kind {
            b'0' | 0 => files.push((name, body.to_vec())),
            b'L' => long_name = Some(text(body)),
            b'x' => long_name = pax_path(body),
            _ => {}
        }
    }
    Ok(files)
}

/// The text of a field of a tar header, up to its first zero byte.
fn text(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// A tar header's number, in octal digits ending with a space or a zero
/// byte.
fn octal(field: &[u8]) -> Result<usize, Unreadable> {
    let digits = text(field);
    let digits = digits.trim_matches(|c: char| c == ' ' || c == '\0');
    match digits {
        "" => Ok(0),
        _ => usize::from_str_radix(digits, 8).map_err(|_| Unreadable),
    }
}

/// The `path` a POSIX tar file's extended header gives the entry after
/// it: records of `<length> <name>=<value>\n`.
fn pax_path(body: &[u8]) -> Option<String> {
    let mut rest = body;
    while !rest.is_empty() {
        let space = rest.iter().position(|&byte| byte == b' ')?;
        let length: usize = std::str::from_utf8(&rest[..space]).ok()?.parse().ok()?;
        let record = rest.get(space + 1..length)?;
        let record = record.strip_suffix(b"\n").unwrap_or(record);
        if let Some(path) = record.strip_prefix(b"path=") {
            return Some(String::from_utf8_lossy(path).into_owned());
        }
        rest = &rest[length..];
    }
    None
}

/// The files of the zip file `bytes`, by its central directory, a name
/// written with backslashes read with forward slashes.
fn zipped(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Unreadable> {
    const END: u32 = 0x0605_4b50;
    const ENTRY: u32 = 0x0201_4b50;
    const LOCAL: u32 = 0x0403_4b50;
    const ENCRYPTED: u16 = 1;
    let shortest = bytes.len().checked_sub(22).ok_or(Unreadable)?;
    // The end record comes last, followed by a comment of up to 64 KB.
    let end = (shortest.saturating_sub(0xffff)..=shortest)
        .rev()
        .find(|&at| le32(bytes, at) == Ok(END))
        .ok_or(Unreadable)?;
    let count = le16(bytes, end + 10)?;
    let mut at = usize::try_from(le32(bytes, end + 16)?).map_err(|_| Unreadable)?;
    let mut files = Vec::new();
    for _ in 0..count {
        if le32(bytes, at)? != ENTRY {
            return Err(Unreadable);
        }
        let flags = le16(bytes, at + 8)?;
        let method = le16(bytes, at + 10)?;
        let packed = le32(bytes, at + 20)? as usize;
        let unpacked = le32(bytes, at + 24)? as usize;
        let name_length = usize::from(le16(bytes, at + 28)?);
        let extra_length = usize::from(le16(bytes, at + 30)?);
        let comment_length = usize::from(le16(bytes, at + 32)?);
        let local = le32(bytes, at + 42)? as usize;
        let name = bytes
            .get(at + 46..at + 46 + name_length)
            .ok_or(Unreadable)?;
        let name = String::from_utf8_lossy(name).replace('\\', "/");
        at += 46 + name_length + extra_length + comment_length;
        if name.ends_with('/') {
            continue;
        }
        if flags & ENCRYPTED != 0 || le32(bytes, local)? != LOCAL {
            return Err(Unreadable);
        }
        let data = local
            + 30
            + usize::from(le16(bytes, local + 26)?)
            + usize::from(le16(bytes, local + 28)?);
        let data = bytes.get(data..data + packed).ok_or(Unreadable)?;
        let file = match method {
            0 => data.to_vec(),
            8 => decompress_to_vec_with_limit(data, unpacked.min(UNPACKED_LIMIT))
                .map_err(|_| Unreadable)?,
            _ => return Err(Unreadable),
        };
        files.push((name, file));
    }
    Ok(files)
}

fn le16(bytes: &[u8], at: usize) -> Result<u16, Unreadable> {
    let field = bytes.get(at..at + 2).ok_or(Unreadable)?;
    Ok(u16::from_le_bytes([field[0], field[1]]))
}

fn le32(bytes: &[u8], at: usize) -> Result<u32, Unreadable> {
    let field = bytes.get(at..at + 4).ok_or(Unreadable)?;
    Ok(u32::from_le_bytes([field[0], field[1], field[2], field[3]]))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A tar file's header for a file named `name` of `size` bytes.
    fn header(name: &str, size: usize, kind: u8) -> Vec<u8> {
        let mut header = vec![0; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000644\0");
        header[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|&byte| u32::from(byte)).sum();
        header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        header
    }

    fn padded(body: &[u8]) -> Vec<u8> {
        let mut body = body.to_vec();
        body.resize(body.len().div_ceil(512) * 512, 0);
        body
    }

    /// A gzipped tar file holding `entries`, each a name, a kind and the
    /// bytes.
    pub(in crate::upgrade) fn tar_gz(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut tar = Vec::new();
        for (name, kind, body) in entries {
            tar.extend(header(name, body.len(), *kind));
            tar.extend(padded(body));
        }
        tar.extend([0; 1024]);
        let mut gz = vec![0x1f, 0x8b, 8, 8, 0, 0, 0, 0, 0, 3];
        gz.extend(b"package.tar\0");
        gz.extend(miniz_oxide::deflate::compress_to_vec(&tar, 6));
        // The check and the length, which nothing here reads.
        gz.extend([0; 8]);
        gz
    }

    /// A zip file holding `files`, the first stored and the rest deflated.
    pub(in crate::upgrade) fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = Vec::new();
        let mut directory = Vec::new();
        for (index, (name, body)) in files.iter().enumerate() {
            let (method, packed) = match index {
                0 => (0u16, body.to_vec()),
                _ => (8, miniz_oxide::deflate::compress_to_vec(body, 6)),
            };
            let local = zip.len() as u32;
            zip.extend(0x0403_4b50u32.to_le_bytes());
            zip.extend([20, 0, 0, 0]);
            zip.extend(method.to_le_bytes());
            zip.extend([0; 8]);
            zip.extend((packed.len() as u32).to_le_bytes());
            zip.extend((body.len() as u32).to_le_bytes());
            zip.extend((name.len() as u16).to_le_bytes());
            zip.extend(0u16.to_le_bytes());
            zip.extend(name.as_bytes());
            zip.extend(&packed);
            directory.extend(0x0201_4b50u32.to_le_bytes());
            directory.extend([20, 0, 20, 0, 0, 0]);
            directory.extend(method.to_le_bytes());
            directory.extend([0; 8]);
            directory.extend((packed.len() as u32).to_le_bytes());
            directory.extend((body.len() as u32).to_le_bytes());
            directory.extend((name.len() as u16).to_le_bytes());
            directory.extend([0; 12]);
            directory.extend(local.to_le_bytes());
            directory.extend(name.as_bytes());
        }
        let start = zip.len() as u32;
        zip.extend(&directory);
        zip.extend(0x0605_4b50u32.to_le_bytes());
        zip.extend([0; 4]);
        zip.extend((files.len() as u16).to_le_bytes());
        zip.extend((files.len() as u16).to_le_bytes());
        zip.extend((directory.len() as u32).to_le_bytes());
        zip.extend(start.to_le_bytes());
        zip.extend([0; 2]);
        zip
    }

    #[test]
    fn the_files_of_a_tar_file_are_read_by_their_paths() {
        let long = format!("statac-0.8.1-target/{}", "a".repeat(120));
        let record = " path=statac-0.8.1-target/pax\n";
        let pax = format!("{}{record}", record.len() + 2);
        let package = tar_gz(&[
            ("statac-0.8.1-target/", b'5', b""),
            ("statac-0.8.1-target/statac", b'0', b"program"),
            ("././@LongLink", b'L', format!("{long}\0").as_bytes()),
            ("ignored", b'0', b"long"),
            ("PaxHeader", b'x', pax.as_bytes()),
            ("ignored", b'0', b"pax"),
            ("statac-0.8.1-target/link", b'2', b""),
        ]);
        assert_eq!(
            files(&package, false),
            Ok(vec![
                ("statac-0.8.1-target/statac".to_owned(), b"program".to_vec()),
                (long, b"long".to_vec()),
                ("statac-0.8.1-target/pax".to_owned(), b"pax".to_vec()),
            ])
        );
    }

    #[test]
    fn the_files_of_a_zip_file_are_read_stored_or_deflated_with_forward_slashes() {
        let package = zip(&[
            ("statac-0.8.1-target\\statac.exe", b"program"),
            ("statac-0.8.1-target\\LICENSE-MIT", &b"licence ".repeat(100)),
        ]);
        assert_eq!(
            files(&package, true),
            Ok(vec![
                (
                    "statac-0.8.1-target/statac.exe".to_owned(),
                    b"program".to_vec()
                ),
                (
                    "statac-0.8.1-target/LICENSE-MIT".to_owned(),
                    b"licence ".repeat(100)
                ),
            ])
        );
    }

    #[test]
    fn what_isnt_a_package_doesnt_read_as_one() {
        let tar = tar_gz(&[("statac", b'0', b"program")]);
        let zip = zip(&[("statac", b"program")]);
        for (bytes, zipped) in [
            (&b""[..], false),
            (&b"not a package at all"[..], false),
            (&tar[..tar.len() / 2], false),
            (&b""[..], true),
            (&zip[..zip.len() - 30], true),
            (&tar[..], true),
        ] {
            assert_eq!(files(bytes, zipped), Err(Unreadable), "{zipped}");
        }
    }
}
