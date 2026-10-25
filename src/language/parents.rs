//! Where a tag's data comes from when Statac has none of its own for the
//! tag itself: the tag the Unicode CLDR's data inherits from.

/// The English regions the CLDR writes as one of its two broader forms of
/// English rather than as `en`: `en-001`, English beyond the United
/// States, and `en-150`, English in Europe.
pub(super) const ENGLISHES: &[(&str, &[&str])] = &[
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
];

/// The tag `tag` inherits from: the broader English the CLDR names for an
/// English region, else the tag without its last part.
pub(super) fn parent(tag: &str) -> Option<&str> {
    let named = ENGLISHES.iter().find(|(_, regions)| {
        regions
            .iter()
            .any(|region| region.eq_ignore_ascii_case(tag))
    });
    match named {
        Some((broader, _)) => Some(broader),
        None => tag.rsplit_once('-').map(|(less, _)| less),
    }
}

/// The tag, then each it inherits from in turn: `en-GB`, `en-001`, `en`.
pub(super) fn inherited(tag: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(tag), |tag| parent(tag))
}
