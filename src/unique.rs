//! Unique identifier truncation and splitting utilities.
//!
//! Provides functions and the [`IdUniqueExt`] trait to compute the minimal unique
//! prefix of an identifier that distinguishes it from a collection of candidate
//! identifiers. All truncation operations respect UTF-8 character boundaries.
//!
//! # Examples
//!
//! ```rust
//! use id_kit::{truncate_id_by_unique, split_id_by_unique, IdUniqueExt};
//!
//! let candidates = vec!["abc100", "abc200", "abc300", "xyz999"];
//!
//! // Top-level function
//! let unique_prefix = truncate_id_by_unique("abc100", 3, &candidates);
//! assert_eq!(unique_prefix, "abc1");
//!
//! // Extension trait
//! let (prefix, remainder) = "abc100".split_id_by_unique(3, &candidates);
//! assert_eq!(prefix, "abc1");
//! assert_eq!(remainder, "00");
//! ```

use crate::identifiable::Identifiable;

/// Snaps `index` up to the next valid UTF-8 char boundary in `s`.
/// If `index >= s.len()`, returns `s.len()`.
pub(crate) fn ceil_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// Computes the minimum byte prefix length required for `target` to uniquely diverge from `candidate`.
/// Uses an internal `is_ascii()` fast-path.
/// If `target == candidate`, returns `target.len()`.
pub(crate) fn divergence_len(target: &str, candidate: &str) -> usize {
    if target == candidate {
        return target.len();
    }

    if target.is_ascii() && candidate.is_ascii() {
        let lcp = target
            .as_bytes()
            .iter()
            .zip(candidate.as_bytes().iter())
            .take_while(|(a, b)| a == b)
            .count();
        if lcp < target.len() {
            lcp + 1
        } else {
            target.len()
        }
    } else {
        let mut lcp_bytes = 0;
        for (ca, cb) in target.chars().zip(candidate.chars()) {
            if ca == cb {
                lcp_bytes += ca.len_utf8();
            } else {
                break;
            }
        }

        if lcp_bytes < target.len() {
            let next_char = target[lcp_bytes..]
                .chars()
                .next()
                .map_or(0, |c| c.len_utf8());
            lcp_bytes + next_char
        } else {
            target.len()
        }
    }
}

/// Truncates the identifier of `target` to the minimal unique prefix that distinguishes it from all `candidates`.
///
/// Candidates with identical IDs (`candidate.id() == target.id()`) are ignored.
/// The prefix is at least `min_len` bytes (snapped up to the next valid UTF-8 char boundary)
/// and clamped to `target.id().len()`.
///
/// # Examples
///
/// ```rust
/// use id_kit::truncate_id_by_unique;
///
/// let target = "4f8b9a123";
/// let candidates = ["4f8b9b456", "4f8a11111", "555555555"];
///
/// // Differentiates from "4f8b9b..." at the 6th char ('a' vs 'b')
/// assert_eq!(truncate_id_by_unique(target, 4, &candidates), "4f8b9a");
/// ```
pub fn truncate_id_by_unique<'a, T: Identifiable + ?Sized, C: Identifiable>(
    target: &'a T,
    min_len: usize,
    candidates: &[C],
) -> &'a str {
    let target_id = target.id();
    let snapped_min = ceil_char_boundary(target_id, min_len.min(target_id.len()));
    let mut max_divergence = snapped_min;

    for candidate in candidates {
        let cand_id = candidate.id();
        if cand_id == target_id {
            continue;
        }
        let div = divergence_len(target_id, cand_id);
        if div > max_divergence {
            max_divergence = div;
        }
    }

    let final_len = max_divergence.min(target_id.len());
    &target_id[..final_len]
}

/// Splits the identifier of `target` into `(unique_prefix, remainder)`.
///
/// Candidates with identical IDs (`candidate.id() == target.id()`) are ignored.
/// The prefix is at least `min_len` bytes (snapped up to the next valid UTF-8 char boundary)
/// and clamped to `target.id().len()`.
///
/// # Examples
///
/// ```rust
/// use id_kit::split_id_by_unique;
///
/// let target = "4f8b9a123";
/// let candidates = ["4f8b9b456", "555555555"];
///
/// let (prefix, remainder) = split_id_by_unique(target, 4, &candidates);
/// assert_eq!(prefix, "4f8b9a");
/// assert_eq!(remainder, "123");
/// ```
pub fn split_id_by_unique<'a, T: Identifiable + ?Sized, C: Identifiable>(
    target: &'a T,
    min_len: usize,
    candidates: &[C],
) -> (&'a str, &'a str) {
    let target_id = target.id();
    let prefix = truncate_id_by_unique(target, min_len, candidates);
    let split_idx = prefix.len();
    (&target_id[..split_idx], &target_id[split_idx..])
}

/// Extension trait providing unique prefix truncation and splitting on any [`Identifiable`] type.
pub trait IdUniqueExt: Identifiable {
    /// Truncates this identifier to the minimal unique prefix among `candidates`.
    fn truncate_id_by_unique<C: Identifiable>(&self, min_len: usize, candidates: &[C]) -> &str;

    /// Splits this identifier into `(unique_prefix, remainder)` among `candidates`.
    fn split_id_by_unique<C: Identifiable>(&self, min_len: usize, candidates: &[C])
        -> (&str, &str);
}

impl<T: Identifiable + ?Sized> IdUniqueExt for T {
    #[inline]
    fn truncate_id_by_unique<C: Identifiable>(&self, min_len: usize, candidates: &[C]) -> &str {
        truncate_id_by_unique(self, min_len, candidates)
    }

    #[inline]
    fn split_id_by_unique<C: Identifiable>(
        &self,
        min_len: usize,
        candidates: &[C],
    ) -> (&str, &str) {
        split_id_by_unique(self, min_len, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceil_char_boundary_ascii() {
        let s = "hello";
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 1), 1);
        assert_eq!(ceil_char_boundary(s, 4), 4);
        assert_eq!(ceil_char_boundary(s, 5), 5);
        assert_eq!(ceil_char_boundary(s, 10), 5);
    }

    #[test]
    fn test_ceil_char_boundary_multibyte() {
        // "가" is 3 bytes (0..3), "나" is 3 bytes (3..6), "다" is 3 bytes (6..9)
        let s = "가나다";
        assert_eq!(s.len(), 9);
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 1), 3);
        assert_eq!(ceil_char_boundary(s, 2), 3);
        assert_eq!(ceil_char_boundary(s, 3), 3);
        assert_eq!(ceil_char_boundary(s, 4), 6);
        assert_eq!(ceil_char_boundary(s, 5), 6);
        assert_eq!(ceil_char_boundary(s, 6), 6);
        assert_eq!(ceil_char_boundary(s, 7), 9);
        assert_eq!(ceil_char_boundary(s, 8), 9);
        assert_eq!(ceil_char_boundary(s, 9), 9);
        assert_eq!(ceil_char_boundary(s, 20), 9);
    }

    #[test]
    fn test_divergence_len_ascii() {
        // Identical
        assert_eq!(divergence_len("abcdef", "abcdef"), 6);

        // Diverging at index 3 ('d' vs 'x')
        assert_eq!(divergence_len("abcdef", "abcxyz"), 4);

        // Target is prefix of candidate
        assert_eq!(divergence_len("abc", "abcdef"), 3);

        // Candidate is prefix of target
        assert_eq!(divergence_len("abcdef", "abc"), 4);

        // Diverging at first char
        assert_eq!(divergence_len("abcdef", "uvwxyz"), 1);

        // Empty strings
        assert_eq!(divergence_len("", ""), 0);
        assert_eq!(divergence_len("", "abc"), 0);
        assert_eq!(divergence_len("abc", ""), 1);
    }

    #[test]
    fn test_divergence_len_multibyte() {
        // Identical
        assert_eq!(divergence_len("가나다", "가나다"), 9);

        // Diverging at 3rd char ("가나다" vs "가나라") -> lcp is "가나" (6 bytes) + "다" (3 bytes) = 9
        assert_eq!(divergence_len("가나다", "가나라"), 9);

        // Target is prefix of candidate
        assert_eq!(divergence_len("가나", "가나다"), 6);

        // Candidate is prefix of target
        assert_eq!(divergence_len("가나다", "가나"), 9);

        // Diverging at first char ("가" vs "나") -> 3 bytes
        assert_eq!(divergence_len("가나다", "나라가"), 3);

        // Mixed ASCII and multibyte
        assert_eq!(divergence_len("id-가나", "id-가라"), 6 + 3);
    }

    #[test]
    fn test_truncate_id_by_unique_ascii() {
        let target = "7a8b9c1d2e";
        let candidates = [
            "7a8b9c3333", // shares "7a8b9c" (6 chars) -> divergence = 7 ("7a8b9c1")
            "7a8b000000", // shares "7a8b" (4 chars) -> divergence = 5
            "9999999999", // shares "" (0 chars) -> divergence = 1
        ];

        // min_len = 4, but candidate shares 6 chars, so prefix must be 7
        assert_eq!(truncate_id_by_unique(target, 4, &candidates), "7a8b9c1");

        // min_len = 8, exceeds required divergence (7), so returns 8
        assert_eq!(truncate_id_by_unique(target, 8, &candidates), "7a8b9c1d");

        // min_len = 20, clamped to target length 10
        assert_eq!(truncate_id_by_unique(target, 20, &candidates), "7a8b9c1d2e");
    }

    #[test]
    fn test_disambiguation_multiple_candidates_varying_lcp() {
        let target = "commit-abc-123";
        let candidates = vec![
            "commit-xyz-999", // LCP "commit-" (7 chars) -> div 8
            "commit-abc-000", // LCP "commit-abc-" (11 chars) -> div 12 ("commit-abc-1")
            "commit-abc-100", // LCP "commit-abc-1" (12 chars) -> div 13 ("commit-abc-12")
            "something-else", // LCP "" (0 chars) -> div 1
        ];

        let prefix = truncate_id_by_unique(target, 0, &candidates);
        assert_eq!(prefix, "commit-abc-12");
    }

    #[test]
    fn test_skipping_identical_and_self_exclusion() {
        let target = "task-42";
        let candidates = vec![
            "task-42", // Identical to target: MUST be ignored
            "task-42", // Duplicate identical: MUST be ignored
            "task-40", // LCP "task-4" (6 chars) -> div 7 ("task-42")
            "task-50", // LCP "task-" (5 chars) -> div 6
        ];

        // With task-40 present, divergence needed is 7 ("task-42")
        assert_eq!(truncate_id_by_unique(target, 2, &candidates), "task-42");

        // If candidates only contain identical targets:
        let only_self = vec!["task-42", "task-42"];
        assert_eq!(truncate_id_by_unique(target, 3, &only_self), "tas");
    }

    #[test]
    fn test_min_len_boundary_conditions() {
        let target = "abcdef";
        let candidates = ["uvwxyz"]; // divergence is 1 ('a')

        // min_len = 0 -> minimal divergence (1)
        assert_eq!(truncate_id_by_unique(target, 0, &candidates), "a");

        // min_len = 3 -> max(1, 3) = 3
        assert_eq!(truncate_id_by_unique(target, 3, &candidates), "abc");

        // min_len = 6 -> full length
        assert_eq!(truncate_id_by_unique(target, 6, &candidates), "abcdef");

        // min_len = 100 -> clamped to target.len() (6)
        assert_eq!(truncate_id_by_unique(target, 100, &candidates), "abcdef");
    }

    #[test]
    fn test_multibyte_utf8_char_boundaries() {
        // Korean ID: "사용자-001" (each Korean char is 3 bytes, hyphen is 1 byte, digits are 1 byte)
        // Byte layout:
        // '사' = [0..3]
        // '용' = [3..6]
        // '자' = [6..9]
        // '-'  = [9..10]
        // '0'  = [10..11]
        // '0'  = [11..12]
        // '1'  = [12..13]
        let target = "사용자-001";
        let candidates = vec!["사용자-002", "관리자-001"];

        // min_len = 1 (inside '사' [0..3]): ceil_char_boundary must snap to 3 ('사')
        // But candidate "사용자-002" shares "사용자-00" (12 bytes), so divergence is 13 ("사용자-001")
        assert_eq!(truncate_id_by_unique(target, 1, &candidates), "사용자-001");

        // Only "관리자-001" as candidate (shares no prefix with target):
        let candidates_other = vec!["관리자-001"];

        // min_len = 0 -> divergence is 3 ('사')
        assert_eq!(truncate_id_by_unique(target, 0, &candidates_other), "사");

        // min_len = 1, 2 -> ceil_char_boundary snaps to 3 -> "사"
        assert_eq!(truncate_id_by_unique(target, 1, &candidates_other), "사");
        assert_eq!(truncate_id_by_unique(target, 2, &candidates_other), "사");
        assert_eq!(truncate_id_by_unique(target, 3, &candidates_other), "사");

        // min_len = 4, 5 -> ceil_char_boundary snaps to 6 -> "사용"
        assert_eq!(truncate_id_by_unique(target, 4, &candidates_other), "사용");
        assert_eq!(truncate_id_by_unique(target, 5, &candidates_other), "사용");
        assert_eq!(truncate_id_by_unique(target, 6, &candidates_other), "사용");
    }

    #[test]
    fn test_empty_candidates() {
        let target = "commit-abcdef123456";
        let empty: Vec<&str> = Vec::new();

        // With empty candidates, prefix length is dictated purely by min_len (snapped to char boundary)
        assert_eq!(truncate_id_by_unique(target, 0, &empty), "");
        assert_eq!(truncate_id_by_unique(target, 7, &empty), "commit-");
        assert_eq!(truncate_id_by_unique(target, 50, &empty), target);

        let multibyte = "안녕-세상";
        // min_len = 2 -> snaps to 3 ("안")
        assert_eq!(truncate_id_by_unique(multibyte, 2, &empty), "안");
    }

    #[test]
    fn test_split_id_by_unique() {
        let target = "issue-12345";
        let candidates = vec!["issue-12399", "issue-99999"];

        // LCP with "issue-12399" is "issue-123" (9 bytes) -> divergence is 10 ("issue-1234")
        let (prefix, remainder) = split_id_by_unique(target, 5, &candidates);
        assert_eq!(prefix, "issue-1234");
        assert_eq!(remainder, "5");
        assert_eq!(format!("{prefix}{remainder}"), target);

        // Multibyte split
        let mb_target = "문서-ABC-1";
        let mb_candidates = vec!["문서-XYZ-2"];
        let (mb_pfx, mb_rem) = split_id_by_unique(mb_target, 3, &mb_candidates);
        assert_eq!(format!("{mb_pfx}{mb_rem}"), mb_target);
        assert!(mb_target.starts_with(mb_pfx));
        assert!(mb_target.ends_with(mb_rem));
    }

    #[test]
    fn test_id_unique_ext_trait_methods() {
        struct DomainEntity {
            key: String,
        }

        impl Identifiable for DomainEntity {
            fn id(&self) -> &str {
                &self.key
            }
        }

        let entity = DomainEntity {
            key: "uuid-1234-5678".to_string(),
        };
        let others = [
            DomainEntity {
                key: "uuid-1234-9999".to_string(),
            },
            DomainEntity {
                key: "uuid-0000-0000".to_string(),
            },
        ];

        // Trait method vs top-level function parity
        let pfx_trait = entity.truncate_id_by_unique(8, &others);
        let pfx_fn = truncate_id_by_unique(&entity, 8, &others);
        assert_eq!(pfx_trait, pfx_fn);
        assert_eq!(pfx_trait, "uuid-1234-5");

        let (split_pfx, split_rem) = entity.split_id_by_unique(8, &others);
        assert_eq!(split_pfx, "uuid-1234-5");
        assert_eq!(split_rem, "678");
        assert_eq!(format!("{split_pfx}{split_rem}"), entity.id());

        // Also test directly on &str
        let str_target = "node-alpha";
        let str_candidates = ["node-beta", "node-gamma"];
        assert_eq!(
            str_target.truncate_id_by_unique(4, &str_candidates),
            "node-a"
        );
        let (s_pfx, s_rem) = str_target.split_id_by_unique(4, &str_candidates);
        assert_eq!(s_pfx, "node-a");
        assert_eq!(s_rem, "lpha");
    }

    #[test]
    fn test_target_is_candidate_prefix_or_contained() {
        // Target is "abc", candidate is "abcdef"
        let target = "abc";
        let candidates = ["abcdef"];
        // divergence_len for ("abc", "abcdef") returns target.len() = 3
        assert_eq!(truncate_id_by_unique(target, 0, &candidates), "abc");

        // Target is "abcdef", candidate is "abc"
        let target2 = "abcdef";
        let candidates2 = ["abc"];
        // divergence_len is 4 ("abcd")
        assert_eq!(truncate_id_by_unique(target2, 0, &candidates2), "abcd");
    }

    #[test]
    fn test_lifetime_tied_to_target_not_candidates() {
        let target = String::from("persistent-target-id");
        let prefix: &str;
        let pfx_split: &str;
        let rem_split: &str;

        {
            let temp_candidates = vec![
                String::from("persistent-target-other"),
                String::from("persistent-candidate"),
            ];
            prefix = truncate_id_by_unique(&target, 5, &temp_candidates);
            let (p, r) = split_id_by_unique(&target, 5, &temp_candidates);
            pfx_split = p;
            rem_split = r;
            // temp_candidates drops here at end of scope
        }

        assert_eq!(prefix, "persistent-target-i");
        assert_eq!(pfx_split, "persistent-target-i");
        assert_eq!(rem_split, "d");
    }
}
