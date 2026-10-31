//! Cases for a check of Ed25519 signatures, shared by the tests of
//! Statac's own check and of the one in `tools/install.ps1`: the test
//! vectors of RFC 8032, section 7.1, as published, which must be
//! accepted, and each changed in a way that must be refused.

/// A public key, a signature of a message, and whether a check accepts
/// it.
pub(crate) struct Case {
    pub(crate) label: String,
    pub(crate) key: Vec<u8>,
    pub(crate) signature: Vec<u8>,
    pub(crate) message: Vec<u8>,
    pub(crate) accepted: bool,
}

/// The order of the group the signatures work in, L, little-endian.
const ORDER: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";

/// Encodings of 32 bytes that are no point: y = 2, which has no x, with
/// either sign; y = p and y = 2^255 - 1, written past p; and y = 1, whose
/// only x is 0, marked odd.
const NOT_POINTS: [&str; 5] = [
    "0200000000000000000000000000000000000000000000000000000000000000",
    "0200000000000000000000000000000000000000000000000000000000000080",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "0100000000000000000000000000000000000000000000000000000000000080",
];

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len() / 2)
        .map(|at| u8::from_str_radix(&hex[at * 2..at * 2 + 2], 16).unwrap())
        .collect()
}

/// The little-endian sum of `a` and `b`, of the same length, which it
/// must fit in.
fn sum(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut carry = 0u16;
    let summed = (a.iter().zip(b))
        .map(|(a, b)| {
            let total = u16::from(*a) + u16::from(*b) + carry;
            carry = total >> 8;
            total as u8
        })
        .collect();
    assert_eq!(carry, 0);
    summed
}

pub(crate) fn cases() -> Vec<Case> {
    let vectors: Vec<Case> = include_str!("rfc8032.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let message = match fields[3] {
                "-" => Vec::new(),
                hex => bytes(hex),
            };
            Case {
                label: fields[0].to_owned(),
                key: bytes(fields[1]),
                signature: bytes(fields[2]),
                message,
                accepted: true,
            }
        })
        .collect();
    assert_eq!(vectors.len(), 5);
    let mut cases = Vec::new();
    let mut case = |label: String, key: &[u8], signature: &[u8], message: &[u8], accepted| {
        cases.push(Case {
            label,
            key: key.to_vec(),
            signature: signature.to_vec(),
            message: message.to_vec(),
            accepted,
        });
    };
    for (at, vector) in vectors.iter().enumerate() {
        let (name, key, signature, message) = (
            &vector.label,
            &vector.key,
            &vector.signature,
            &vector.message,
        );
        case(name.to_string(), key, signature, message, true);
        let mut changed = signature.clone();
        changed[0] ^= 1;
        case(format!("{name}, R changed"), key, &changed, message, false);
        let mut changed = signature.clone();
        changed[32] ^= 1;
        case(format!("{name}, S changed"), key, &changed, message, false);
        let other = &vectors[(at + 1) % vectors.len()].key;
        case(
            format!("{name}, another key"),
            other,
            signature,
            message,
            false,
        );
        let mut changed = message.clone();
        match changed.first_mut() {
            Some(first) => *first ^= 1,
            None => changed.push(0),
        }
        case(
            format!("{name}, message changed"),
            key,
            signature,
            &changed,
            false,
        );
        let mut changed = signature.clone();
        changed[32..].copy_from_slice(&sum(&signature[32..], &bytes(ORDER)));
        case(format!("{name}, S + L"), key, &changed, message, false);
    }
    let first = &vectors[0];
    let (name, key, signature, message) =
        (&first.label, &first.key, &first.signature, &first.message);
    for not_point in NOT_POINTS {
        let not_point = bytes(not_point);
        case(
            format!("{name}, key {}", hex(&not_point)),
            &not_point,
            signature,
            message,
            false,
        );
        let mut changed = signature.clone();
        changed[..32].copy_from_slice(&not_point);
        case(
            format!("{name}, R {}", hex(&not_point)),
            key,
            &changed,
            message,
            false,
        );
    }
    cases
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
