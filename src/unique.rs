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
//! let unique_prefix = truncate_id_by_unique("abc100", 3, &candidates, |s| *s);
//! assert_eq!(unique_prefix, "abc1");
//!
//! // Extension trait
//! let (prefix, remainder) = "abc100".split_id_by_unique(3, &candidates, |s| *s);
//! assert_eq!(prefix, "abc1");
//! assert_eq!(remainder, "00");
//! ```

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

#[inline]
fn compute_unique_prefix_len<'a, T, I, K, F>(
    target: &str,
    min_len: usize,
    candidates: I,
    key_fn: F,
) -> usize
where
    T: 'a,
    I: IntoIterator<Item = &'a T>,
    F: Fn(&'a T) -> K,
    K: AsRef<str>,
{
    let snapped_min = ceil_char_boundary(target, min_len.min(target.len()));
    let mut max_divergence = snapped_min;

    for candidate in candidates {
        let cand_key = key_fn(candidate);
        let cand_id = cand_key.as_ref();
        if cand_id == target {
            continue;
        }
        let div = divergence_len(target, cand_id);
        if div > max_divergence {
            max_divergence = div;
        }
    }

    max_divergence.min(target.len())
}

/// Truncates the identifier of `target` to the minimal unique prefix that distinguishes it from all `candidates`.
///
/// Candidates with identical IDs (`cand_id == target.as_ref()`) are ignored.
/// The prefix is at least `min_len` bytes (snapped up to the next valid UTF-8 char boundary)
/// and clamped to `target.as_ref().len()`.
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
/// assert_eq!(truncate_id_by_unique(target, 4, &candidates, |s| *s), "4f8b9a");
/// ```
#[inline]
pub fn truncate_id_by_unique<'a, T, I, K, F>(
    target: impl AsRef<str>,
    min_len: usize,
    candidates: I,
    key_fn: F,
) -> String
where
    T: 'a,
    I: IntoIterator<Item = &'a T>,
    F: Fn(&'a T) -> K,
    K: AsRef<str>,
{
    let target_ref = target.as_ref();
    let split_idx = compute_unique_prefix_len(target_ref, min_len, candidates, key_fn);
    target_ref[..split_idx].to_string()
}

/// Splits the identifier of `target` into `(unique_prefix, remainder)`.
///
/// Candidates with identical IDs (`cand_id == target.as_ref()`) are ignored.
/// The prefix is at least `min_len` bytes (snapped up to the next valid UTF-8 char boundary)
/// and clamped to `target.as_ref().len()`.
///
/// # Examples
///
/// ```rust
/// use id_kit::split_id_by_unique;
///
/// let target = "4f8b9a123";
/// let candidates = ["4f8b9b456", "555555555"];
///
/// let (prefix, remainder) = split_id_by_unique(target, 4, &candidates, |s| *s);
/// assert_eq!(prefix, "4f8b9a");
/// assert_eq!(remainder, "123");
/// ```
#[inline]
pub fn split_id_by_unique<'a, T, I, K, F>(
    target: impl AsRef<str>,
    min_len: usize,
    candidates: I,
    key_fn: F,
) -> (String, String)
where
    T: 'a,
    I: IntoIterator<Item = &'a T>,
    F: Fn(&'a T) -> K,
    K: AsRef<str>,
{
    let target_ref = target.as_ref();
    let split_idx = compute_unique_prefix_len(target_ref, min_len, candidates, key_fn);
    (
        target_ref[..split_idx].to_string(),
        target_ref[split_idx..].to_string(),
    )
}

/// Extension trait providing unique prefix truncation and splitting on any string-like type.
pub trait IdUniqueExt {
    /// Truncates this identifier to the minimal unique prefix among `candidates`.
    fn truncate_id_by_unique<'a, T, I, K, F>(
        &self,
        min_len: usize,
        candidates: I,
        key_fn: F,
    ) -> String
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
        F: Fn(&'a T) -> K,
        K: AsRef<str>;

    /// Splits this identifier into `(unique_prefix, remainder)` among `candidates`.
    fn split_id_by_unique<'a, T, I, K, F>(
        &self,
        min_len: usize,
        candidates: I,
        key_fn: F,
    ) -> (String, String)
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
        F: Fn(&'a T) -> K,
        K: AsRef<str>;
}

impl<S: AsRef<str> + ?Sized> IdUniqueExt for S {
    #[inline]
    fn truncate_id_by_unique<'a, T, I, K, F>(
        &self,
        min_len: usize,
        candidates: I,
        key_fn: F,
    ) -> String
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
        F: Fn(&'a T) -> K,
        K: AsRef<str>,
    {
        truncate_id_by_unique(self, min_len, candidates, key_fn)
    }

    #[inline]
    fn split_id_by_unique<'a, T, I, K, F>(
        &self,
        min_len: usize,
        candidates: I,
        key_fn: F,
    ) -> (String, String)
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
        F: Fn(&'a T) -> K,
        K: AsRef<str>,
    {
        split_id_by_unique(self, min_len, candidates, key_fn)
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
        assert_eq!(
            truncate_id_by_unique(target, 4, &candidates, |s| *s),
            "7a8b9c1"
        );

        // min_len = 8, exceeds required divergence (7), so returns 8
        assert_eq!(
            truncate_id_by_unique(target, 8, &candidates, |s| *s),
            "7a8b9c1d"
        );

        // min_len = 20, clamped to target length 10
        assert_eq!(
            truncate_id_by_unique(target, 20, &candidates, |s| *s),
            "7a8b9c1d2e"
        );
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

        let prefix = truncate_id_by_unique(target, 0, &candidates, |s| *s);
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
        assert_eq!(
            truncate_id_by_unique(target, 2, &candidates, |s| *s),
            "task-42"
        );

        // If candidates only contain identical targets:
        let only_self = vec!["task-42", "task-42"];
        assert_eq!(truncate_id_by_unique(target, 3, &only_self, |s| *s), "tas");
    }

    #[test]
    fn test_min_len_boundary_conditions() {
        let target = "abcdef";
        let candidates = ["uvwxyz"]; // divergence is 1 ('a')

        // min_len = 0 -> minimal divergence (1)
        assert_eq!(truncate_id_by_unique(target, 0, &candidates, |s| *s), "a");

        // min_len = 3 -> max(1, 3) = 3
        assert_eq!(truncate_id_by_unique(target, 3, &candidates, |s| *s), "abc");

        // min_len = 6 -> full length
        assert_eq!(
            truncate_id_by_unique(target, 6, &candidates, |s| *s),
            "abcdef"
        );

        // min_len = 100 -> clamped to target.len() (6)
        assert_eq!(
            truncate_id_by_unique(target, 100, &candidates, |s| *s),
            "abcdef"
        );
    }

    #[test]
    fn test_multibyte_utf8_char_boundaries() {
        // Korean ID: "사용자-001" (each Korean char is 3 bytes, hyphen is 1 byte, digits are 1 byte)
        let target = "사용자-001";
        let candidates = vec!["사용자-002", "관리자-001"];

        // min_len = 1 (inside '사' [0..3]): ceil_char_boundary must snap to 3 ('사')
        // But candidate "사용자-002" shares "사용자-00" (12 bytes), so divergence is 13 ("사용자-001")
        assert_eq!(
            truncate_id_by_unique(target, 1, &candidates, |s| *s),
            "사용자-001"
        );

        // Only "관리자-001" as candidate (shares no prefix with target):
        let candidates_other = vec!["관리자-001"];

        // min_len = 0 -> divergence is 3 ('사')
        assert_eq!(
            truncate_id_by_unique(target, 0, &candidates_other, |s| *s),
            "사"
        );

        // min_len = 1, 2 -> ceil_char_boundary snaps to 3 -> "사"
        assert_eq!(
            truncate_id_by_unique(target, 1, &candidates_other, |s| *s),
            "사"
        );
        assert_eq!(
            truncate_id_by_unique(target, 2, &candidates_other, |s| *s),
            "사"
        );
        assert_eq!(
            truncate_id_by_unique(target, 3, &candidates_other, |s| *s),
            "사"
        );

        // min_len = 4, 5 -> ceil_char_boundary snaps to 6 -> "사용"
        assert_eq!(
            truncate_id_by_unique(target, 4, &candidates_other, |s| *s),
            "사용"
        );
        assert_eq!(
            truncate_id_by_unique(target, 5, &candidates_other, |s| *s),
            "사용"
        );
        assert_eq!(
            truncate_id_by_unique(target, 6, &candidates_other, |s| *s),
            "사용"
        );
    }

    #[test]
    fn test_empty_candidates() {
        let target = "commit-abcdef123456";
        let empty: Vec<&str> = Vec::new();

        // With empty candidates, prefix length is dictated purely by min_len (snapped to char boundary)
        assert_eq!(truncate_id_by_unique(target, 0, &empty, |s| *s), "");
        assert_eq!(truncate_id_by_unique(target, 7, &empty, |s| *s), "commit-");
        assert_eq!(truncate_id_by_unique(target, 50, &empty, |s| *s), target);

        let multibyte = "안녕-세상";
        // min_len = 2 -> snaps to 3 ("안")
        assert_eq!(truncate_id_by_unique(multibyte, 2, &empty, |s| *s), "안");
    }

    #[test]
    fn test_split_id_by_unique() {
        let target = "issue-12345";
        let candidates = vec!["issue-12399", "issue-99999"];

        // LCP with "issue-12399" is "issue-123" (9 bytes) -> divergence is 10 ("issue-1234")
        let (prefix, remainder) = split_id_by_unique(target, 5, &candidates, |s| *s);
        assert_eq!(prefix, "issue-1234");
        assert_eq!(remainder, "5");
        assert_eq!(format!("{prefix}{remainder}"), target);

        // Multibyte split
        let mb_target = "문서-ABC-1";
        let mb_candidates = vec!["문서-XYZ-2"];
        let (mb_pfx, mb_rem) = split_id_by_unique(mb_target, 3, &mb_candidates, |s| *s);
        assert_eq!(format!("{mb_pfx}{mb_rem}"), mb_target);
        assert!(mb_target.starts_with(&mb_pfx));
        assert!(mb_target.ends_with(&mb_rem));
    }

    #[test]
    fn test_id_unique_ext_trait_methods() {
        struct DomainEntity {
            key: String,
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
        let pfx_trait = entity.key.truncate_id_by_unique(8, &others, |e| &e.key);
        let pfx_fn = truncate_id_by_unique(&entity.key, 8, &others, |e| &e.key);
        assert_eq!(pfx_trait, pfx_fn);
        assert_eq!(pfx_trait, "uuid-1234-5");

        let (split_pfx, split_rem) = entity.key.split_id_by_unique(8, &others, |e| &e.key);
        assert_eq!(split_pfx, "uuid-1234-5");
        assert_eq!(split_rem, "678");
        assert_eq!(format!("{split_pfx}{split_rem}"), entity.key);

        // Also test directly on &str
        let str_target = "node-alpha";
        let str_candidates = ["node-beta", "node-gamma"];
        assert_eq!(
            str_target.truncate_id_by_unique(4, &str_candidates, |s| *s),
            "node-a"
        );
        let (s_pfx, s_rem) = str_target.split_id_by_unique(4, &str_candidates, |s| *s);
        assert_eq!(s_pfx, "node-a");
        assert_eq!(s_rem, "lpha");
    }

    #[test]
    fn test_target_is_candidate_prefix_or_contained() {
        // Target is "abc", candidate is "abcdef"
        let target = "abc";
        let candidates = ["abcdef"];
        // divergence_len for ("abc", "abcdef") returns target.len() = 3
        assert_eq!(truncate_id_by_unique(target, 0, &candidates, |s| *s), "abc");

        // Target is "abcdef", candidate is "abc"
        let target2 = "abcdef";
        let candidates2 = ["abc"];
        // divergence_len is 4 ("abcd")
        assert_eq!(
            truncate_id_by_unique(target2, 0, &candidates2, |s| *s),
            "abcd"
        );
    }

    #[test]
    fn test_temporary_target_and_candidates_drop() {
        let prefix: String;
        let pfx_split: String;
        let rem_split: String;

        {
            let temp_target = String::from("persistent-target-id");
            let temp_candidates = vec![
                String::from("persistent-target-other"),
                String::from("persistent-candidate"),
            ];
            prefix = truncate_id_by_unique(&temp_target, 5, &temp_candidates, |s| s.as_str());
            let (p, r) = split_id_by_unique(&temp_target, 5, &temp_candidates, |s| s.as_str());
            pfx_split = p;
            rem_split = r;
            // temp_target and temp_candidates drop here at end of scope
        }

        assert_eq!(prefix, "persistent-target-i");
        assert_eq!(pfx_split, "persistent-target-i");
        assert_eq!(rem_split, "d");
    }

    #[test]
    fn test_temporary_target_without_local_binding() {
        let candidates = vec!["target-100", "target-200"];
        let truncated = format!("target-{}", 100).truncate_id_by_unique(5, &candidates, |s| *s);
        assert_eq!(truncated, "target-1");

        let (prefix, remainder) =
            format!("target-{}", 100).split_id_by_unique(5, &candidates, |s| *s);
        assert_eq!(prefix, "target-1");
        assert_eq!(remainder, "00");
    }

    #[test]
    fn test_temporary_owned_key_value_type_id() {
        struct Node {
            uuid: u128,
        }

        let nodes = [
            Node {
                uuid: 0x1234_5678_0001,
            },
            Node {
                uuid: 0x1234_5678_0002,
            },
            Node {
                uuid: 0x9999_0000_0000,
            },
        ];

        let target_uuid = format!("{:012x}", nodes[0].uuid);
        let truncated =
            target_uuid.truncate_id_by_unique(4, &nodes, |n| format!("{:012x}", n.uuid));
        assert_eq!(truncated, "123456780001");

        let (prefix, remainder) =
            split_id_by_unique(&target_uuid, 4, &nodes, |n| format!("{:012x}", n.uuid));
        assert_eq!(prefix, "123456780001");
        assert_eq!(remainder, "");
    }

    #[test]
    fn test_borrowed_string_field() {
        struct Item {
            id: String,
            _name: String,
        }

        let items = vec![
            Item {
                id: "item-alpha-1".to_string(),
                _name: "Alpha 1".to_string(),
            },
            Item {
                id: "item-alpha-2".to_string(),
                _name: "Alpha 2".to_string(),
            },
            Item {
                id: "item-beta-1".to_string(),
                _name: "Beta 1".to_string(),
            },
        ];

        let truncated = items[0]
            .id
            .truncate_id_by_unique(5, &items, |item| &item.id);
        assert_eq!(truncated, "item-alpha-1");

        let (prefix, remainder) = items[2].id.split_id_by_unique(5, &items, |item| &item.id);
        assert_eq!(prefix, "item-b");
        assert_eq!(remainder, "eta-1");
    }

    #[test]
    fn test_string_slice_candidates() {
        let candidates = vec!["abc", "abd"];
        let truncated = "abc".truncate_id_by_unique(1, &candidates, |s| *s);
        assert_eq!(truncated, "abc");

        let truncated2 = "abx".truncate_id_by_unique(1, &candidates, |s| *s);
        assert_eq!(truncated2, "abx");

        let (p, r) = "abx".split_id_by_unique(1, &candidates, |s| *s);
        assert_eq!(p, "abx");
        assert_eq!(r, "");
    }

    #[test]
    fn test_utf8_multibyte_and_boundary_ceiling() {
        let target = "한글식별자-01";
        let candidates = vec!["한글식별자-02", "영문식별자-01"];

        assert_eq!(
            target.truncate_id_by_unique(2, &candidates, |s| *s),
            "한글식별자-01"
        );

        let candidates2 = vec!["영문식별자-01"];
        assert_eq!(target.truncate_id_by_unique(1, &candidates2, |s| *s), "한");
        assert_eq!(target.truncate_id_by_unique(2, &candidates2, |s| *s), "한");
        assert_eq!(target.truncate_id_by_unique(3, &candidates2, |s| *s), "한");
        assert_eq!(
            target.truncate_id_by_unique(4, &candidates2, |s| *s),
            "한글"
        );
        assert_eq!(
            target.truncate_id_by_unique(5, &candidates2, |s| *s),
            "한글"
        );
        assert_eq!(
            target.truncate_id_by_unique(6, &candidates2, |s| *s),
            "한글"
        );

        let (prefix, remainder) = target.split_id_by_unique(4, &candidates2, |s| *s);
        assert_eq!(prefix, "한글");
        assert_eq!(remainder, "식별자-01");
    }
}
