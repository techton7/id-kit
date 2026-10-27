//! # uid-prefix
//!
//! Fast, zero-dependency Git-style short UID resolution, LCP collision disambiguation,
//! and dynamic prefix abbreviation in Rust.
//!
//! Designed for CLIs, databases, and developer tools operating on UUIDs, ULIDs,
//! Git hashes, or any string-based unique identifiers.
//!
//! ## Features
//! - **Git-style Short Prefix Resolution**: Resolves case-insensitive prefixes against candidate collections.
//! - **Pairwise LCP Collision Diagnostics**: When a prefix collision occurs, computes the exact pairwise Longest Common Prefix (LCP) and tells the caller the minimum character length required to disambiguate.
//! - **Dynamic ID Abbreviation**: Pre-computes the shortest collision-free prefix for every ID in a collection in $O(N \log N)$ time.
//! - **Zero Dependencies**: Pure Rust standard library by default. Optional `serde` feature available.
//!
//! ## Quick Start
//!
//! ```rust
//! use uid_prefix::{resolve_prefix, UidAbbreviator, PrefixError};
//!
//! let uids = vec![
//!     "01M5VDRGAAAA",
//!     "01M5VDRGBBBB",
//!     "01M5VDV4CCCC",
//! ];
//!
//! // 1. Unique match (8 characters or more)
//! let matched = resolve_prefix("01M5VDV4", &uids).unwrap();
//! assert_eq!(*matched, "01M5VDV4CCCC");
//!
//! // 2. Collision detection with exact required length
//! match resolve_prefix("01M5VDRG", &uids) {
//!     Err(PrefixError::Ambiguous { required_len, candidates, .. }) => {
//!         assert_eq!(required_len, 9); // Collided up to 8 chars ("01M5VDRG"), requires 9 ("01M5VDRGA" vs "01M5VDRGB")
//!         assert_eq!(candidates.len(), 2);
//!     }
//!     _ => unreachable!(),
//! }
//!
//! // 3. Abbreviate IDs to their shortest unambiguous prefix
//! let abbr = UidAbbreviator::from_ids(uids.iter().copied());
//! assert_eq!(abbr.abbreviate("01M5VDV4CCCC"), "01M5VDV4");
//! ```

pub mod abbreviator;
pub mod error;
pub mod resolver;

pub use abbreviator::UidAbbreviator;
pub use error::{AmbiguousCandidate, PrefixError};
pub use resolver::{
    common_prefix_len, resolve_prefix, resolve_prefix_by, resolve_prefix_with,
    DEFAULT_MIN_PREFIX_LEN,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_prefix_unique_match() {
        let ids = vec![
            "0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b",
            "0194b2c3-4d5e-5f6a-9b0c-1d2e3f4a5b6c",
            "0194c3d4-5e6f-6a7b-0c1d-2e3f4a5b6c7d",
        ];

        // Match with lowercase
        let res = resolve_prefix("0194a1b2", &ids).unwrap();
        assert_eq!(*res, "0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b");

        // Case-insensitive match with uppercase
        let res_upper = resolve_prefix("0194B2C3", &ids).unwrap();
        assert_eq!(*res_upper, "0194b2c3-4d5e-5f6a-9b0c-1d2e3f4a5b6c");
    }

    #[test]
    fn test_resolve_prefix_too_short() {
        let ids = vec!["0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b"];
        let err = resolve_prefix("0194a1b", &ids).unwrap_err();
        match err {
            PrefixError::TooShort {
                prefix,
                actual_len,
                min_len,
            } => {
                assert_eq!(prefix, "0194a1b");
                assert_eq!(actual_len, 7);
                assert_eq!(min_len, 8);
            }
            _ => panic!("Expected TooShort error"),
        }
    }

    #[test]
    fn test_resolve_prefix_not_found() {
        let ids = vec!["0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b"];
        let err = resolve_prefix("0194ffff", &ids).unwrap_err();
        assert!(matches!(err, PrefixError::NotFound { .. }));
    }

    #[test]
    fn test_resolve_prefix_ambiguous_collision_calculates_required_len() {
        // Two IDs sharing 10 characters "0194a1b2-3"
        let id1 = "0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b";
        let id2 = "0194a1b2-3f9a-4e5f-8a9b-0c1d2e3f4a5b";
        let id3 = "0194ffff-0000-0000-0000-000000000000";
        let ids = vec![id1, id2, id3];

        // 8-character prefix matches both id1 and id2
        let err = resolve_prefix("0194a1b2", &ids).unwrap_err();
        match err {
            PrefixError::Ambiguous {
                prefix,
                required_len,
                candidates,
            } => {
                assert_eq!(prefix, "0194a1b2");
                // Common prefix is "0194a1b2-3" (10 chars), so required_len is 11
                assert_eq!(required_len, 11);
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].disambiguated_prefix, "0194a1b2-3c");
                assert_eq!(candidates[1].disambiguated_prefix, "0194a1b2-3f");
            }
            _ => panic!("Expected Ambiguous error"),
        }

        // Resolving with 11 characters disambiguates id1
        let resolved = resolve_prefix("0194a1b2-3c", &ids).unwrap();
        assert_eq!(*resolved, id1);
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TaskItem {
        uid: String,
        title: String,
    }

    impl std::fmt::Display for TaskItem {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Task '{}'", self.title)
        }
    }

    #[test]
    fn test_resolve_prefix_by_with_domain_objects() {
        let tasks = vec![
            TaskItem {
                uid: "01M5VDRG2E2DVBXJST67N1JWPD".into(),
                title: "v0.1.0 Root".into(),
            },
            TaskItem {
                uid: "01M5VDRPDYPZ471ZZ1X13CYQQF".into(),
                title: "Timerit Core Spec".into(),
            },
        ];

        let err = resolve_prefix_by("01M5VDR", &tasks, 7, |t| &t.uid).unwrap_err();
        match err {
            PrefixError::Ambiguous {
                required_len,
                candidates,
                ..
            } => {
                assert_eq!(required_len, 8);
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].item.title, "v0.1.0 Root");
                assert_eq!(candidates[0].disambiguated_prefix, "01M5VDRG");
                assert_eq!(candidates[1].item.title, "Timerit Core Spec");
                assert_eq!(candidates[1].disambiguated_prefix, "01M5VDRP");
            }
            _ => panic!("Expected Ambiguous error"),
        }

        let resolved = resolve_prefix_by("01M5VDRP", &tasks, 7, |t| &t.uid).unwrap();
        assert_eq!(resolved.title, "Timerit Core Spec");
    }

    #[test]
    fn test_uid_abbreviator() {
        let uids = [
            "01M5VDRG2E2DVBXJST67N1JWPD",
            "01M5VDRPDYPZ471ZZ1X13CYQQF",
            "01M5VDV4NY1NSKDK3XEQ10QYB7",
        ];

        let abbr = UidAbbreviator::from_ids(uids.iter().copied());

        // id0 and id1 share 7 characters ("01M5VDR"), so need max(8, 7+1) = 8
        assert_eq!(abbr.abbreviate(uids[0]), "01M5VDRG");
        assert_eq!(abbr.abbreviate(uids[1]), "01M5VDRP");

        // id2 shares 6 characters with id1 ("01M5VD"), so needs max(8, 6+1) = 8
        assert_eq!(abbr.abbreviate(uids[2]), "01M5VDV4");
    }

    #[test]
    fn test_uid_abbreviator_with_custom_min_len() {
        let uids = ["abc11111", "abc22222"];
        let abbr = UidAbbreviator::with_min_len(uids.iter().copied(), 4);
        // Common prefix is "abc" (3 chars), min_len is 4. LCP + 1 = 4.
        assert_eq!(abbr.abbreviate(uids[0]), "abc1");
        assert_eq!(abbr.abbreviate(uids[1]), "abc2");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_serialization() {
        let err: PrefixError<String> = PrefixError::Ambiguous {
            prefix: "0194a1b2".into(),
            required_len: 11,
            candidates: vec![AmbiguousCandidate {
                item: "Task 1".into(),
                id: "0194a1b2-3c4d".into(),
                disambiguated_prefix: "0194a1b2-3c".into(),
            }],
        };

        let json = serde_json::to_string(&err).unwrap();
        let deserialized: PrefixError<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }
}
