//! Names made from content: `site.css` published as `site.1a2b3c4d.css`,
//! so a browser holding an old copy is handed the new one the moment a
//! byte changes.

/// The first 8 hex digits of the content's BLAKE3 hash.
pub(super) fn mark(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex()[..8].to_owned()
}

/// `name` with `mark` in front of its file's extension.
pub(super) fn marked(name: &str, mark: &str) -> String {
    let file_start = name.rfind('/').map_or(0, |slash| slash + 1);
    match name[file_start..].rfind('.').filter(|&dot| dot > 0) {
        Some(dot) => {
            let (stem, extension) = name.split_at(file_start + dot);
            format!("{stem}.{mark}{extension}")
        }
        None => format!("{name}.{mark}"),
    }
}

/// The groups of files that name each other in a ring, each group after
/// every group its members name. A file in no ring is a group of one.
/// `names[i]` lists the files file `i` names.
pub(super) fn rings(names: &[Vec<usize>]) -> Vec<Vec<usize>> {
    // Tarjan's algorithm, with its own stack so a long chain of files
    // can't overflow the thread's.
    const UNSEEN: usize = usize::MAX;
    let count = names.len();
    let mut order = vec![UNSEEN; count];
    let mut low = vec![0; count];
    let mut held = vec![false; count];
    let mut stack = Vec::new();
    let mut rings = Vec::new();
    let mut next = 0;
    for start in 0..count {
        if order[start] != UNSEEN {
            continue;
        }
        let mut walk = vec![(start, 0)];
        order[start] = next;
        low[start] = next;
        next += 1;
        stack.push(start);
        held[start] = true;
        while let Some(&mut (at, ref mut edge)) = walk.last_mut() {
            if let Some(&to) = names[at].get(*edge) {
                *edge += 1;
                if order[to] == UNSEEN {
                    order[to] = next;
                    low[to] = next;
                    next += 1;
                    stack.push(to);
                    held[to] = true;
                    walk.push((to, 0));
                } else if held[to] {
                    low[at] = low[at].min(order[to]);
                }
                continue;
            }
            walk.pop();
            if let Some(&(from, _)) = walk.last() {
                low[from] = low[from].min(low[at]);
            }
            if low[at] == order[at] {
                let mut ring = Vec::new();
                while let Some(member) = stack.pop() {
                    held[member] = false;
                    ring.push(member);
                    if member == at {
                        break;
                    }
                }
                ring.sort_unstable();
                rings.push(ring);
            }
        }
    }
    rings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_goes_in_front_of_the_extension() {
        assert_eq!(marked("site.css", "1a2b3c4d"), "site.1a2b3c4d.css");
        assert_eq!(
            marked("js/lib.min.js", "1a2b3c4d"),
            "js/lib.min.1a2b3c4d.js"
        );
        assert_eq!(marked("v1.2/LICENSE", "1a2b3c4d"), "v1.2/LICENSE.1a2b3c4d");
        assert_eq!(marked(".htaccess", "1a2b3c4d"), ".htaccess.1a2b3c4d");
    }

    #[test]
    fn the_mark_is_eight_hex_digits_of_the_content() {
        let one = mark(b"body {}");
        assert_eq!(one.len(), 8);
        assert!(one.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(one, &blake3::hash(b"body {}").to_hex()[..8]);
        assert_ne!(one, mark(b"body { margin: 0 }"));
    }

    #[test]
    fn named_files_come_first_and_a_ring_comes_whole() {
        // 0 names 1 and 2; 1 names 3; 2 and 4 name each other; 5 names itself.
        let names = [vec![1, 2], vec![3], vec![4], vec![], vec![2, 3], vec![5]];
        let rings = rings(&names);
        assert_eq!(rings.len(), 5);
        assert!(rings.contains(&vec![2, 4]));
        let position = |file| rings.iter().position(|ring| ring.contains(&file)).unwrap();
        for (from, to) in [(0, 1), (0, 2), (1, 3), (4, 3)] {
            assert!(position(to) < position(from), "{rings:?}");
        }
    }

    #[test]
    fn a_long_chain_is_walked_without_overflowing() {
        let names: Vec<Vec<usize>> = (0..100_000).map(|i| vec![i + 1]).chain([vec![]]).collect();
        let rings = rings(&names);
        assert_eq!(rings.len(), names.len());
        assert_eq!(rings[0], [100_000]);
    }
}
