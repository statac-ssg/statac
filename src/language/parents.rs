//! Where a tag's data comes from when Statac has none of its own for the
//! tag itself: the tag the Unicode CLDR's data inherits from.

/// The regions the CLDR writes as a broader form of their language rather
/// than as the language itself, by that form: `en-001`, English beyond the
/// United States, `en-150`, English in Europe, and `es-419`, Spanish in
/// Latin America.
pub(super) const PARENTS: &[(&str, &[&str])] = &[
    (
        "en-001",
        &[
            "en-150", "en-AG", "en-AI", "en-AU", "en-BB", "en-BM", "en-BS", "en-BW", "en-BZ",
            "en-CC", "en-CK", "en-CM", "en-CX", "en-CY", "en-DG", "en-DM", "en-ER", "en-FJ",
            "en-FK", "en-FM", "en-GB", "en-GD", "en-GG", "en-GH", "en-GI", "en-GM", "en-GS",
            "en-GY", "en-HK", "en-ID", "en-IE", "en-IL", "en-IM", "en-IN", "en-IO", "en-JE",
            "en-JM", "en-KE", "en-KI", "en-KN", "en-KY", "en-LC", "en-LR", "en-LS", "en-MG",
            "en-MO", "en-MS", "en-MT", "en-MU", "en-MV", "en-MW", "en-MY", "en-NA", "en-NF",
            "en-NG", "en-NR", "en-NU", "en-NZ", "en-PG", "en-PK", "en-PN", "en-PW", "en-RW",
            "en-SB", "en-SC", "en-SD", "en-SG", "en-SH", "en-SL", "en-SS", "en-SX", "en-SZ",
            "en-TC", "en-TK", "en-TO", "en-TT", "en-TV", "en-TZ", "en-UG", "en-VC", "en-VG",
            "en-VU", "en-WS", "en-ZA", "en-ZM", "en-ZW",
        ],
    ),
    (
        "en-150",
        &[
            "en-AT", "en-BE", "en-CH", "en-CZ", "en-DE", "en-DK", "en-EE", "en-ES", "en-FI",
            "en-FR", "en-GE", "en-HU", "en-IT", "en-LT", "en-LV", "en-NL", "en-NO", "en-PL",
            "en-PT", "en-RO", "en-SE", "en-SI", "en-SK", "en-UA",
        ],
    ),
    (
        "es-419",
        &[
            "es-AR", "es-BO", "es-BR", "es-BZ", "es-CL", "es-CO", "es-CR", "es-CU", "es-DO",
            "es-EC", "es-GT", "es-HN", "es-JP", "es-MX", "es-NI", "es-PA", "es-PE", "es-PR",
            "es-PY", "es-SV", "es-US", "es-UY", "es-VE",
        ],
    ),
];

/// The script each language Statac has words for is written in where a
/// tag names none, as the CLDR's likely subtags give it.
pub(super) const SCRIPTS: &[(&str, &str)] = &[
    ("ar", "Arab"),
    ("de", "Latn"),
    ("en", "Latn"),
    ("es", "Latn"),
    ("fa", "Arab"),
    ("fr", "Latn"),
    ("ja", "Jpan"),
    ("ko", "Kore"),
];

/// The tag `tag` inherits from: none for a language and a script other
/// than its usual one, `en-Dsrt`, whose CLDR data inherits from no
/// language; the broader form of its language the CLDR names for a
/// region; else the tag without its last part.
pub(super) fn parent(tag: &str) -> Option<String> {
    if tag.matches('-').count() == 1 && in_another_script(tag) {
        return None;
    }
    let named = PARENTS.iter().find(|(_, regions)| {
        regions
            .iter()
            .any(|region| region.eq_ignore_ascii_case(tag))
    });
    match named {
        Some((broader, _)) => Some((*broader).to_owned()),
        None => tag.rsplit_once('-').map(|(less, _)| less.to_owned()),
    }
}

/// Whether `tag` names a language Statac has words for and, next, a script
/// other than its usual one.
fn in_another_script(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let (Some(language), Some(script)) = (parts.next(), parts.next()) else {
        return false;
    };
    SCRIPTS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(language))
        .is_some_and(|(_, usual)| is_script(script) && !usual.eq_ignore_ascii_case(script))
}

fn is_script(part: &str) -> bool {
    part.len() == 4 && part.bytes().all(|b| b.is_ascii_alphabetic())
}

/// `tag` without the script it names, where that isn't another script
/// than its language's usual one: `en-Latn-GB` as `en-GB`.
fn without_script(tag: &str) -> Option<String> {
    let mut parts = tag.splitn(3, '-');
    let (language, script) = (parts.next()?, parts.next()?);
    if !is_script(script) || in_another_script(tag) {
        return None;
    }
    Some(match parts.next() {
        Some(rest) => format!("{language}-{rest}"),
        None => language.to_owned(),
    })
}

/// The tag, then the tag without its script and each it inherits from,
/// then each the tag itself inherits from, every one once at its last
/// place: `en-Latn-GB`, `en-GB`, `en-001`, `en-Latn`, `en`.
pub(super) fn inherited(tag: &str) -> impl Iterator<Item = String> {
    let chain = |from: String| std::iter::successors(Some(from), |tag| parent(tag));
    let all: Vec<String> = std::iter::once(tag.to_owned())
        .chain(without_script(tag).into_iter().flat_map(chain))
        .chain(chain(tag.to_owned()).skip(1))
        .collect();
    let mut once: Vec<String> = Vec::with_capacity(all.len());
    for tag in all.into_iter().rev() {
        if !once.iter().any(|known| known.eq_ignore_ascii_case(&tag)) {
            once.push(tag);
        }
    }
    once.into_iter().rev()
}
