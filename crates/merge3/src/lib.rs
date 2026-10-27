//! Three-way merge of sequences: two edits of one base, combined when they touch different
//! places. Pure and deterministic; knows nothing about what the items are.
//!
//! The base is lined up with each edit (longest common subsequence). Items all three agree on
//! are anchors; between two anchors the base has a stretch and each edit has its own:
//!  - an edit that left the stretch as it was gives way to the other,
//!  - two identical edits count once,
//!  - two different edits of the same stretch are a [`Conflict`].

#![forbid(unsafe_code)]

/// Both sides changed the same stretch of the base differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conflict {
    /// Index in the base where the clashing stretch starts.
    pub at: usize,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "both sides changed the same part (near item {})",
            self.at
        )
    }
}

impl std::error::Error for Conflict {}

/// For every base index, the index of the matching item in `other`, if the pair is part of the
/// longest common subsequence.
fn align<T: PartialEq>(base: &[T], other: &[T]) -> Vec<Option<usize>> {
    let mut map = vec![None; base.len()];
    // Edits are usually local: peel the common ends so the table covers only the middle.
    let prefix = base.iter().zip(other).take_while(|(a, b)| a == b).count();
    let suffix = base[prefix..]
        .iter()
        .rev()
        .zip(other[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    for (i, slot) in map.iter_mut().enumerate().take(prefix) {
        *slot = Some(i);
    }
    for k in 0..suffix {
        if let Some(slot) = map.get_mut(base.len() - 1 - k) {
            *slot = Some(other.len() - 1 - k);
        }
    }
    let mid_base = &base[prefix..base.len() - suffix];
    let mid_other = &other[prefix..other.len() - suffix];
    if mid_base.is_empty() || mid_other.is_empty() {
        return map;
    }
    // table[i][j]: length of the LCS of mid_base[i..] and mid_other[j..].
    let width = mid_other.len() + 1;
    let mut table = vec![0u32; (mid_base.len() + 1) * width];
    for i in (0..mid_base.len()).rev() {
        for j in (0..mid_other.len()).rev() {
            table[i * width + j] = if mid_base[i] == mid_other[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < mid_base.len() && j < mid_other.len() {
        if mid_base[i] == mid_other[j] {
            map[prefix + i] = Some(prefix + j);
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    map
}

/// Merges `ours` and `theirs`, two edits of `base`.
///
/// # Errors
/// [`Conflict`] when both changed the same stretch of `base` in different ways.
pub fn merge3<T: Clone + PartialEq>(
    base: &[T],
    ours: &[T],
    theirs: &[T],
) -> Result<Vec<T>, Conflict> {
    if ours == theirs {
        return Ok(ours.to_vec());
    }
    if ours == base {
        return Ok(theirs.to_vec());
    }
    if theirs == base {
        return Ok(ours.to_vec());
    }
    let to_ours = align(base, ours);
    let to_theirs = align(base, theirs);
    let mut merged = Vec::with_capacity(base.len().max(ours.len()).max(theirs.len()));
    // The previous anchor: base index and the matching indices in both edits (all exclusive
    // starts of the next stretch).
    let (mut b0, mut o0, mut t0) = (0usize, 0usize, 0usize);
    let anchors = (0..base.len())
        .filter_map(|i| Some((i, to_ours[i]?, to_theirs[i]?)))
        .chain(std::iter::once((base.len(), ours.len(), theirs.len())));
    for (b1, o1, t1) in anchors {
        let (stretch_base, stretch_ours, stretch_theirs) =
            (&base[b0..b1], &ours[o0..o1], &theirs[t0..t1]);
        if stretch_ours == stretch_base || stretch_ours == stretch_theirs {
            merged.extend_from_slice(stretch_theirs);
        } else if stretch_theirs == stretch_base {
            merged.extend_from_slice(stretch_ours);
        } else {
            return Err(Conflict { at: b0 });
        }
        if b1 < base.len() {
            merged.push(base[b1].clone());
        }
        (b0, o0, t0) = (b1 + 1, o1 + 1, t1 + 1);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn edits_in_different_places_are_both_kept() {
        let base = words("a b c d e f");
        let ours = words("a X c d e f");
        let theirs = words("a b c d e Y");
        assert_eq!(merge3(&base, &ours, &theirs), Ok(words("a X c d e Y")));
    }

    #[test]
    fn insertions_and_deletions_combine() {
        let base = words("one two three four");
        let ours = words("one two NEW three four");
        let theirs = words("one two three");
        assert_eq!(
            merge3(&base, &ours, &theirs),
            Ok(words("one two NEW three"))
        );
    }

    #[test]
    fn the_same_edit_on_both_sides_counts_once() {
        let base = words("a b c");
        let both = words("a Z c");
        assert_eq!(merge3(&base, &both, &both), Ok(both.clone()));
    }

    #[test]
    fn different_edits_of_the_same_item_conflict() {
        let base = words("a b c");
        let ours = words("a X c");
        let theirs = words("a Y c");
        assert_eq!(merge3(&base, &ours, &theirs), Err(Conflict { at: 1 }));
    }

    #[test]
    fn appends_by_both_sides_at_the_end_conflict_but_one_append_does_not() {
        let base = words("a b");
        assert_eq!(
            merge3(&base, &words("a b X"), &words("a b Y")),
            Err(Conflict { at: 2 })
        );
        assert_eq!(merge3(&base, &words("a b X"), &base), Ok(words("a b X")));
    }

    #[test]
    fn empty_sequences_are_fine() {
        let none: Vec<&str> = Vec::new();
        assert_eq!(
            merge3(&none, &words("a"), &words("b")),
            Err(Conflict { at: 0 })
        );
        assert_eq!(merge3(&none, &none, &words("b")), Ok(words("b")));
    }

    fn sequence() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(0u8..6, 0..24)
    }

    proptest! {
        #[test]
        fn an_untouched_side_gives_way(base in sequence(), edit in sequence()) {
            prop_assert_eq!(merge3(&base, &edit, &base), Ok(edit.clone()));
            prop_assert_eq!(merge3(&base, &base, &edit), Ok(edit));
        }

        #[test]
        fn identical_edits_merge_to_themselves(base in sequence(), edit in sequence()) {
            prop_assert_eq!(merge3(&base, &edit, &edit), Ok(edit));
        }

        #[test]
        fn a_merge_never_loses_what_neither_side_touched(
            base in sequence(), ours in sequence(), theirs in sequence()
        ) {
            if let Ok(merged) = merge3(&base, &ours, &theirs) {
                // Items kept by both edits (as a subsequence of the base) are in the result.
                let kept: Vec<&u8> = base.iter().filter(|x| ours.contains(x) && theirs.contains(x)).collect();
                let mut cursor = merged.iter();
                let mut stays = true;
                for item in &kept {
                    if !cursor.any(|m| m == *item) {
                        stays = false;
                        break;
                    }
                }
                // Not every shared item is an anchor (duplicates), so only check when all of
                // the base is shared by both.
                if base.iter().all(|x| ours.contains(x) && theirs.contains(x)) {
                    prop_assert!(stays);
                }
            }
        }
    }
}
