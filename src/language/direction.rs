//! Which way the lines of a language run, for a language Statac has no
//! words for: that of the script the Unicode CLDR gives as the usual one
//! for the language, or of the script its tag names.

use super::Direction;

/// The scripts written right to left, by their ISO 15924 codes.
pub(super) const SCRIPTS: &[&str] = &[
    "Adlm", "Arab", "Armi", "Avst", "Chrs", "Cprt", "Elym", "Gara", "Hatr", "Hebr", "Hung", "Khar",
    "Lydi", "Mand", "Mani", "Mend", "Merc", "Mero", "Narb", "Nbat", "Nkoo", "Orkh", "Ougr", "Palm",
    "Phli", "Phlp", "Phnx", "Prti", "Rohg", "Samr", "Sarb", "Sidt", "Sogd", "Sogo", "Syrc", "Thaa",
    "Yezi",
];

/// The languages whose usual script is one of `SCRIPTS`.
pub(super) const LANGUAGES: &[&str] = &[
    "aao", "abh", "abv", "acm", "acq", "acw", "acx", "adf", "ae", "aeb", "aec", "aee", "aeq",
    "afb", "aib", "aii", "aij", "aiq", "amw", "apc", "apd", "ar", "arc", "arq", "ars", "ary",
    "arz", "ask", "atn", "auj", "auz", "avd", "avl", "ayh", "ayl", "ayn", "ayp", "azb", "bal",
    "bdz", "bej", "bft", "bgn", "bgp", "bhe", "bhm", "bhn", "bjf", "bjm", "bqi", "brh", "brk",
    "bsh", "bsk", "chg", "cja", "ckb", "clh", "czk", "dcc", "def", "deh", "dgl", "dmk", "dml",
    "dv", "ecy", "esh", "fa", "fay", "faz", "fia", "fub", "gbz", "ggg", "gha", "ghr", "gig", "gjk",
    "gju", "glh", "glk", "grr", "gwc", "gwf", "gwt", "gzi", "hac", "haz", "hbo", "he", "hkh",
    "hnd", "hno", "hoh", "hrt", "hrz", "hss", "huy", "isk", "itk", "iw", "jad", "jat", "jbe",
    "jbn", "jdg", "ji", "jnd", "jog", "jpa", "jpr", "jrb", "jye", "kbu", "kby", "kcy", "kfm",
    "khw", "klj", "kmz", "kqd", "ks", "ktl", "kvx", "kxp", "lad", "lah", "lhs", "lki", "lrc",
    "lrk", "lrl", "lsa", "lsd", "lss", "luv", "luz", "mby", "mde", "mey", "mfa", "mfi", "mhj",
    "mid", "mki", "mnj", "mve", "mvy", "myz", "mzb", "mzn", "nli", "nlm", "nqo", "ntz", "nyq",
    "oar", "obm", "odk", "oru", "ota", "otk", "oui", "pal", "pbt", "pgd", "phl", "phn", "phr",
    "phv", "plk", "prc", "prd", "prx", "ps", "psh", "psi", "pst", "qxq", "rdb", "rhg", "rmt",
    "sam", "sbn", "scl", "sd", "sdb", "sdf", "sdg", "sdh", "sds", "sgr", "sgy", "shd", "shm",
    "shu", "shv", "siy", "siz", "skr", "smp", "smy", "sog", "sqo", "sqt", "srh", "srz", "ssh",
    "sts", "swb", "syc", "syn", "syr", "tjo", "tks", "tmr", "tov", "tra", "trg", "trm", "trw",
    "ug", "ur", "ush", "uzs", "vaf", "vgr", "vmh", "wbk", "wlo", "wne", "wni", "wsv", "xco", "xhe",
    "xka", "xkc", "xkj", "xkp", "xld", "xly", "xmn", "xmr", "xna", "xpr", "xsa", "xsd", "xvi",
    "ydg", "yhd", "yi", "yih", "yud", "zba", "zdj", "zrp", "zum",
];

/// A language with a region where its usual script runs the other way
/// from the language's elsewhere.
pub(super) const REGIONS: &[&str] = &[
    "az-IQ", "az-IR", "ha-CM", "ha-SD", "kk-AF", "kk-CN", "kk-IR", "kk-MN", "ku-IQ", "ku-IR",
    "ku-LB", "ky-CN", "mey-SN", "ms-CC", "pa-PK", "sd-IN", "tg-PK", "ug-KZ", "ug-MN", "uz-AF",
];

impl Direction {
    /// Which way the language `tag` names is written: by its script
    /// where it names one (`pa-Arab`), by its region where the language's
    /// script differs there (`pa-PK`), and by its language otherwise.
    pub(crate) fn of(tag: &str) -> Direction {
        let mut parts = tag.split('-');
        let language = parts.next().unwrap_or_default();
        let (mut script, mut region) = (None, None);
        for part in parts {
            let letters = part.bytes().all(|b| b.is_ascii_alphabetic());
            let figures = part.bytes().all(|b| b.is_ascii_digit());
            match part.len() {
                4 if letters => script = script.or(Some(part)),
                2 if letters => region = region.or(Some(part)),
                3 if figures => region = region.or(Some(part)),
                _ => {}
            }
        }
        let among = |known: &[&str], text: &str| known.iter().any(|k| k.eq_ignore_ascii_case(text));
        let right_to_left = match (script, region) {
            (Some(script), _) => among(SCRIPTS, script),
            (None, Some(region)) => {
                among(LANGUAGES, language) != among(REGIONS, &format!("{language}-{region}"))
            }
            (None, None) => among(LANGUAGES, language),
        };
        if right_to_left {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        }
    }
}
