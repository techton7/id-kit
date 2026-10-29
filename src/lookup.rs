//! Identifier prefix lookup and resolution utilities.
//!
//! This module provides functions and extension traits to resolve an item from a
//! collection of candidates using a prefix of its identifier.
//!
//! - [`find_by_id_prefix`]: Top-level function for resolving by prefix.
//! - [`IdSliceExt`]: Extension trait implemented for slices `[T]`, providing `.find_by_id_prefix(prefix, key_fn)`.
//!
//! Resolution succeeds with [`Ok`] when exactly one candidate matches the prefix.
//! If zero candidates match, [`ResolveError::NotFound`] is returned.
//! If two or more candidates match, [`ResolveError::Ambiguous`] is returned along with
//! the list of conflicting candidate IDs and the minimum prefix length needed to disambiguate them.
//!
//! # Examples
//!
//! ```rust
//! use id_kit::{find_by_id_prefix, IdSliceExt};
//!
//! let candidates = vec!["commit-1a2b", "commit-3c4d", "branch-main"];
//!
//! // Top-level function
//! let found = find_by_id_prefix(&candidates, "commit-1", |s| *s).unwrap();
//! assert_eq!(*found, "commit-1a2b");
//!
//! // Extension trait on slice
//! let found = candidates.find_by_id_prefix("branch", |s| *s).unwrap();
//! assert_eq!(*found, "branch-main");
//! ```

use crate::error::ResolveError;
use crate::unique::divergence_len;

/// Finds a single item by prefix matching its identifier extracted via `key_fn`.
///
/// Matches using exact case-sensitive `starts_with`.
///
/// # Errors
/// - `ResolveError::NotFound` if no candidates match `prefix`.
/// - `ResolveError::Ambiguous` if multiple candidates match `prefix`.
///   The `required_len` field indicates the minimum length needed to disambiguate the candidates.
pub fn find_by_id_prefix<'a, T, K, F>(
    candidates: &'a [T],
    prefix: &str,
    key_fn: F,
) -> Result<&'a T, ResolveError>
where
    F: Fn(&'a T) -> K,
    K: AsRef<str>,
{
    let mut matches: Vec<(&'a T, String)> = Vec::new();

    for candidate in candidates {
        let key = key_fn(candidate);
        let key_str = key.as_ref();
        if key_str.starts_with(prefix) {
            matches.push((candidate, key_str.to_string()));
        }
    }

    match matches.len() {
        0 => Err(ResolveError::NotFound {
            prefix: prefix.to_string(),
        }),
        1 => Ok(matches[0].0),
        _ => {
            let mut max_divergence = prefix.len();
            for i in 0..matches.len() {
                for j in (i + 1)..matches.len() {
                    let id_i = &matches[i].1;
                    let id_j = &matches[j].1;
                    let div_ij = divergence_len(id_i, id_j);
                    let div_ji = divergence_len(id_j, id_i);
                    max_divergence = max_divergence.max(div_ij).max(div_ji);
                }
            }

            let matching_ids: Vec<String> = matches.into_iter().map(|(_, id)| id).collect();

            Err(ResolveError::Ambiguous {
                prefix: prefix.to_string(),
                candidates: matching_ids,
                required_len: max_divergence,
            })
        }
    }
}

/// Extension trait for slices providing prefix-based identifier resolution.
pub trait IdSliceExt<T> {
    /// Finds a single item whose identifier starts with `prefix`.
    ///
    /// # Errors
    /// - `ResolveError::NotFound` if no candidates match `prefix`.
    /// - `ResolveError::Ambiguous` if multiple candidates match `prefix`.
    fn find_by_id_prefix<'a, K, F>(
        &'a self,
        prefix: &str,
        key_fn: F,
    ) -> Result<&'a T, ResolveError>
    where
        T: 'a,
        F: Fn(&'a T) -> K,
        K: AsRef<str>;
}

impl<T> IdSliceExt<T> for [T] {
    #[inline]
    fn find_by_id_prefix<'a, K, F>(&'a self, prefix: &str, key_fn: F) -> Result<&'a T, ResolveError>
    where
        T: 'a,
        F: Fn(&'a T) -> K,
        K: AsRef<str>,
    {
        find_by_id_prefix(self, prefix, key_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct MockItem {
        id: String,
        description: String,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct StaticItem {
        key: &'static str,
        value: i32,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Node {
        uuid: u128,
        label: &'static str,
    }

    #[test]
    fn test_single_match_ascii_and_domain_structs() {
        // ASCII &str
        let strings = ["item-apple", "item-banana", "other-cherry"];
        let res = find_by_id_prefix(&strings, "item-a", |s| *s);
        assert_eq!(res, Ok(&"item-apple"));

        // Domain struct with borrowed field
        let items = [
            MockItem {
                id: "uuid-1111".to_string(),
                description: "First".to_string(),
            },
            MockItem {
                id: "uuid-2222".to_string(),
                description: "Second".to_string(),
            },
        ];
        let found = find_by_id_prefix(&items, "uuid-1", |item| &item.id);
        assert_eq!(
            found,
            Ok(&MockItem {
                id: "uuid-1111".to_string(),
                description: "First".to_string(),
            })
        );

        // Domain struct with static str field
        let statics = [
            StaticItem {
                key: "node-alpha",
                value: 10,
            },
            StaticItem {
                key: "node-beta",
                value: 20,
            },
        ];
        let found = find_by_id_prefix(&statics, "node-b", |item| item.key);
        assert_eq!(
            found,
            Ok(&StaticItem {
                key: "node-beta",
                value: 20,
            })
        );
    }

    #[test]
    fn test_not_found_when_no_item_starts_with_prefix() {
        let candidates = ["apple", "banana", "cherry"];
        let err = find_by_id_prefix(&candidates, "date", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::NotFound {
                prefix: "date".to_string(),
            }
        );

        // Also test partial mismatch (prefix exists in substring but not start)
        let err2 = find_by_id_prefix(&candidates, "anana", |s| *s).unwrap_err();
        assert_eq!(
            err2,
            ResolveError::NotFound {
                prefix: "anana".to_string(),
            }
        );
    }

    #[test]
    fn test_not_found_on_empty_candidate_slice() {
        let empty_slice: &[&str] = &[];
        let err = find_by_id_prefix(empty_slice, "anything", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::NotFound {
                prefix: "anything".to_string(),
            }
        );

        let empty_vec: Vec<MockItem> = Vec::new();
        let err_vec = find_by_id_prefix(&empty_vec, "prefix", |item| &item.id).unwrap_err();
        assert_eq!(
            err_vec,
            ResolveError::NotFound {
                prefix: "prefix".to_string(),
            }
        );
    }

    #[test]
    fn test_ambiguous_two_or_more_items() {
        let candidates = ["commit-100", "commit-101", "commit-200", "commit-300"];
        let err = find_by_id_prefix(&candidates, "commit-10", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "commit-10".to_string(),
                candidates: vec!["commit-100".to_string(), "commit-101".to_string()],
                required_len: 10,
            }
        );

        // Ambiguous across 3 candidates with different divergences
        let items = ["node-10a", "node-10b", "node-10c", "node-200"];
        let err3 = find_by_id_prefix(&items, "node-1", |s| *s).unwrap_err();
        assert_eq!(
            err3,
            ResolveError::Ambiguous {
                prefix: "node-1".to_string(),
                candidates: vec![
                    "node-10a".to_string(),
                    "node-10b".to_string(),
                    "node-10c".to_string()
                ],
                required_len: 8,
            }
        );
    }

    #[test]
    fn test_exact_case_sensitivity() {
        let candidates = ["ABC-01", "abc-02", "Abc-03"];

        // "abc" should match only "abc-02"
        let res_lower = find_by_id_prefix(&candidates, "abc", |s| *s);
        assert_eq!(res_lower, Ok(&"abc-02"));

        // "ABC" should match only "ABC-01"
        let res_upper = find_by_id_prefix(&candidates, "ABC", |s| *s);
        assert_eq!(res_upper, Ok(&"ABC-01"));

        // "Abc" should match only "Abc-03"
        let res_title = find_by_id_prefix(&candidates, "Abc", |s| *s);
        assert_eq!(res_title, Ok(&"Abc-03"));

        // "aBC" does not exist -> NotFound
        let res_none = find_by_id_prefix(&candidates, "aBC", |s| *s);
        assert_eq!(
            res_none,
            Err(ResolveError::NotFound {
                prefix: "aBC".to_string(),
            })
        );
    }

    #[test]
    fn test_multibyte_utf8_prefix_matching() {
        // Korean test cases
        let candidates = ["사용자-001", "사용자-002", "관리자-001"];

        // Single unique prefix match
        let found = find_by_id_prefix(&candidates, "관리", |s| *s);
        assert_eq!(found, Ok(&"관리자-001"));

        // Ambiguous match with Korean characters
        // "사용자-001" and "사용자-002": LCP is "사용자-00" (3*3 + 1 + 2 = 12 bytes), divergence requires 13 bytes
        let err = find_by_id_prefix(&candidates, "사용", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "사용".to_string(),
                candidates: vec!["사용자-001".to_string(), "사용자-002".to_string()],
                required_len: 13,
            }
        );

        // Accented characters (e.g. French / Italian)
        let accents = ["café-latte", "café-mocha", "caffè-macchiato"];
        let found_accent = find_by_id_prefix(&accents, "caffè", |s| *s);
        assert_eq!(found_accent, Ok(&"caffè-macchiato"));

        let err_accent = find_by_id_prefix(&accents, "café", |s| *s).unwrap_err();
        assert_eq!(
            err_accent,
            ResolveError::Ambiguous {
                prefix: "café".to_string(),
                candidates: vec!["café-latte".to_string(), "café-mocha".to_string()],
                // "café-" has 3 ('caf') + 2 ('é') + 1 ('-') = 6 bytes.
                // Next char 'l' or 'm' is 1 byte -> required_len = 7 bytes ("café-l").
                required_len: 7,
            }
        );
    }

    #[test]
    fn test_trait_extension_method_vs_top_level_function() {
        let candidates = vec!["deploy-prod", "deploy-staging", "test-unit"];

        // Top-level function
        let res_fn = find_by_id_prefix(&candidates, "deploy-p", |s| *s);
        // Extension method on slice (&[T])
        let res_trait = candidates.find_by_id_prefix("deploy-p", |s| *s);

        assert_eq!(res_fn, res_trait);
        assert_eq!(res_trait, Ok(&"deploy-prod"));

        // Error cases via extension method
        assert_eq!(
            candidates.find_by_id_prefix("nonexistent", |s| *s),
            find_by_id_prefix(&candidates, "nonexistent", |s| *s)
        );
        assert_eq!(
            candidates.find_by_id_prefix("deploy-", |s| *s),
            find_by_id_prefix(&candidates, "deploy-", |s| *s)
        );
    }

    #[test]
    fn test_full_id_exact_match() {
        let candidates = ["item-10", "item-20", "other-30"];

        // Prefix exactly equals "item-10"
        let found = find_by_id_prefix(&candidates, "item-10", |s| *s);
        assert_eq!(found, Ok(&"item-10"));

        // If another item has "item-10" as prefix, then full ID match is ambiguous
        let candidates_with_nested = ["item-10", "item-10-sub"];
        let err = find_by_id_prefix(&candidates_with_nested, "item-10", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "item-10".to_string(),
                candidates: vec!["item-10".to_string(), "item-10-sub".to_string()],
                required_len: 8,
            }
        );
    }

    #[test]
    fn test_disambiguation_nested_prefixes() {
        let candidates = ["prefix-a", "prefix-ab", "prefix-xyz"];

        let err = find_by_id_prefix(&candidates, "prefix-a", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "prefix-a".to_string(),
                candidates: vec!["prefix-a".to_string(), "prefix-ab".to_string()],
                required_len: 9,
            }
        );
    }

    #[test]
    fn test_empty_prefix_behavior() {
        // Empty prefix matches everything starting with ""
        let single = ["single-item"];
        assert_eq!(find_by_id_prefix(&single, "", |s| *s), Ok(&"single-item"));

        let multiple = ["alpha", "beta"];
        let err = find_by_id_prefix(&multiple, "", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "".to_string(),
                candidates: vec!["alpha".to_string(), "beta".to_string()],
                required_len: 1, // 'a' vs 'b' diverges at 1
            }
        );
    }

    #[test]
    fn test_identical_duplicate_ids() {
        let candidates = ["dup-id", "dup-id"];
        let err = find_by_id_prefix(&candidates, "dup", |s| *s).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "dup".to_string(),
                candidates: vec!["dup-id".to_string(), "dup-id".to_string()],
                required_len: 6,
            }
        );
    }

    #[test]
    fn test_exact_prefix_match_borrowed_field() {
        let nodes = [
            MockItem {
                id: "node-100".to_string(),
                description: "Node 1".to_string(),
            },
            MockItem {
                id: "node-200".to_string(),
                description: "Node 2".to_string(),
            },
            MockItem {
                id: "node-300".to_string(),
                description: "Node 3".to_string(),
            },
        ];

        let res = find_by_id_prefix(&nodes, "node-2", |n| &n.id);
        assert_eq!(res, Ok(&nodes[1]));

        let slice_res = nodes.find_by_id_prefix("node-3", |n| &n.id);
        assert_eq!(slice_res, Ok(&nodes[2]));
    }

    #[test]
    fn test_value_type_id_simulation_temporary_owned_string() {
        let nodes = [
            Node {
                uuid: 0x1234abcd,
                label: "first",
            },
            Node {
                uuid: 0x1234cdef,
                label: "second",
            },
            Node {
                uuid: 0x99990000,
                label: "third",
            },
        ];

        // Match single candidate via temporary owned String
        let found = find_by_id_prefix(&nodes, "1234a", |n| format!("{:x}", n.uuid));
        assert_eq!(found, Ok(&nodes[0]));

        // Ambiguous match with temporary owned String
        let err = find_by_id_prefix(&nodes, "1234", |n| format!("{:x}", n.uuid)).unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                prefix: "1234".to_string(),
                candidates: vec!["1234abcd".to_string(), "1234cdef".to_string()],
                // "1234a" vs "1234c" diverges at length 5
                required_len: 5,
            }
        );

        // Extension method equivalence with temporary owned String
        let ext_found = nodes.find_by_id_prefix("1234a", |n| format!("{:x}", n.uuid));
        assert_eq!(ext_found, Ok(&nodes[0]));
    }

    #[test]
    fn test_string_slices_deref() {
        let candidates = ["alpha", "beta", "gamma"];

        let found = find_by_id_prefix(&candidates, "al", |s| *s);
        assert_eq!(found, Ok(&"alpha"));

        let found_ext = candidates.find_by_id_prefix("be", |s| *s);
        assert_eq!(found_ext, Ok(&"beta"));
    }

    #[test]
    fn test_ambiguous_calculates_accurate_required_len() {
        let candidates = ["service-auth-v1", "service-auth-v2", "service-billing"];
        let err = find_by_id_prefix(&candidates, "service-a", |s| *s).unwrap_err();
        match err {
            ResolveError::Ambiguous {
                prefix,
                candidates: matched,
                required_len,
            } => {
                assert_eq!(prefix, "service-a");
                assert_eq!(
                    matched,
                    vec!["service-auth-v1".to_string(), "service-auth-v2".to_string()]
                );
                // Divergence between "service-auth-v1" and "service-auth-v2":
                // LCP is "service-auth-v" (14 chars). Divergence is 15.
                assert_eq!(required_len, 15);
                assert_eq!(divergence_len("service-auth-v1", "service-auth-v2"), 15);
            }
            ResolveError::NotFound { .. } => panic!("expected Ambiguous, got NotFound"),
        }
    }
}
