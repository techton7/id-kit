//! Integration and edge-case test suite for Phase 2 operations in `id-kit`.
//!
//! Verifies:
//! - AC-1: Round-trip consistency across diverse realistic ID ecosystems.
//! - Ambiguity disambiguation guarantee with `required_len`.
//! - AC-2: Lifetime independence of truncated/split slices from candidates.
//! - AC-3: Complex UTF-8 multi-byte scenarios (Korean, Emoji, 2/3/4-byte code points, boundaries).
//! - Custom domain structs using `impl_identifiable!`.
//! - Boundary conditions and edge cases (single candidate, duplicates, empty list, bounds).

use id_kit::{
    find_by_id_prefix, impl_identifiable, split_id_by_unique, truncate_id_by_unique, IdSliceExt,
    IdUniqueExt, Identifiable, ResolveError,
};

// ============================================================================
// Test Domain Fixtures & Structs
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
struct Commit {
    hash: String,
    message: String,
}

impl_identifiable!(Commit, hash);

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeployRecord<'a> {
    id: &'a str,
    environment: &'static str,
}

impl_identifiable!(DeployRecord<'a>, id);

#[derive(Debug, Clone, PartialEq, Eq)]
struct IssueTrackerItem {
    key: String,
    priority: u32,
}

impl Identifiable for IssueTrackerItem {
    fn id(&self) -> &str {
        &self.key
    }
}

/// Realistic Git SHA-1 hashes (40 hex characters)
const GIT_SHA1_SAMPLES: &[&str] = &[
    "e4d909c290d0fb1ca068ffaddf22cbd0add8b16a",
    "e4d909c390d0fb1ca068ffaddf22cbd0add8b16b",
    "e4d909a090d0fb1ca068ffaddf22cbd0add8b16c",
    "e4d9100090d0fb1ca068ffaddf22cbd0add8b16d",
    "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
    "a1b2c3d4e5f60718293a4b5c6d7e8f9087654321",
    "ffffffffffffffffffffffffffffffffffffff01",
    "ffffffffffffffffffffffffffffffffffffff02",
    "0000000000000000000000000000000000000001",
    "0000000000000000000000000000000000000002",
];

/// Realistic Git SHA-256 hashes (64 hex characters)
const GIT_SHA256_SAMPLES: &[&str] = &[
    "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae",
    "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7af",
    "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e800",
    "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8",
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
];

/// Realistic UUIDv4 values (36 chars, hyphenated)
const UUID_SAMPLES: &[&str] = &[
    "550e8400-e29b-41d4-a716-446655440000",
    "550e8400-e29b-41d4-a716-446655440001",
    "550e8400-e29b-41d4-a716-446655440099",
    "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
    "a8098c1a-f86e-11da-bd1a-00112444be1e",
    "f47ac10b-58cc-4372-a567-0e02b2c3d479",
];

/// Realistic ULID samples (26 Crockford Base32 characters)
const ULID_SAMPLES: &[&str] = &[
    "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    "01ARZ3NDEKTSV4RRFFQ69G5FB0",
    "01ARZ3NDEKTSV4RRFFQ69G5FC1",
    "01ARZ3NDEKTSV4RRFFQ69G5FD2",
    "01BX5ZZKBKACTAV9WEVGEMMVRY",
    "7ZZZZZZZZZZZZZZZZZZZZZZZZZ",
];

/// Realistic NanoID samples (21 characters)
const NANOID_SAMPLES: &[&str] = &[
    "V1StGXR8_Z5jdHi6B-myT",
    "V1StGXR8_Z5jdHi6B-myU",
    "V1StGXR8_Z5jdHi6B-myV",
    "4f3nz9LkceA-FaU0Ij_4p",
    "ynm89xK_44mK8wR3j01q9",
    "zZaB019283-abcdefghij",
];

// ============================================================================
// 1. Round-Trip Consistency (AC-1)
// ============================================================================

fn verify_round_trip_for_collection<T: Identifiable + std::fmt::Debug>(
    collection: &[T],
    min_lens: &[usize],
) {
    for &min_len in min_lens {
        for (idx, item) in collection.iter().enumerate() {
            let short = item.truncate_id_by_unique(min_len, collection);
            assert!(
                item.id().starts_with(short),
                "Truncated prefix '{short}' must be a prefix of target '{}'",
                item.id()
            );

            let (split_prefix, remainder) = item.split_id_by_unique(min_len, collection);
            assert_eq!(
                split_prefix,
                short,
                "split prefix must match truncate prefix for '{}'",
                item.id()
            );
            assert_eq!(
                format!("{split_prefix}{remainder}"),
                item.id(),
                "Recombining prefix and remainder must recreate exact target ID"
            );

            let lookup_result = collection.find_by_id_prefix(short);
            assert!(
                lookup_result.is_ok(),
                "Round-trip failed: looking up prefix '{short}' (min_len={min_len}, item[{idx}]='{}') returned error: {:?}",
                item.id(),
                lookup_result.err()
            );
            let resolved = lookup_result.unwrap();
            assert_eq!(
                resolved.id(),
                item.id(),
                "Resolved ID '{}' must match original ID '{}'",
                resolved.id(),
                item.id()
            );
        }
    }
}

#[test]
fn test_round_trip_git_sha1() {
    let min_lens = [0, 1, 4, 7, 8, 12, 20, 40, 100];
    verify_round_trip_for_collection(GIT_SHA1_SAMPLES, &min_lens);
}

#[test]
fn test_round_trip_git_sha256() {
    let min_lens = [0, 4, 7, 12, 32, 64, 128];
    verify_round_trip_for_collection(GIT_SHA256_SAMPLES, &min_lens);
}

#[test]
fn test_round_trip_uuids() {
    let min_lens = [0, 4, 8, 14, 20, 36, 50];
    verify_round_trip_for_collection(UUID_SAMPLES, &min_lens);
}

#[test]
fn test_round_trip_ulids() {
    let min_lens = [0, 4, 10, 18, 26, 60];
    verify_round_trip_for_collection(ULID_SAMPLES, &min_lens);
}

#[test]
fn test_round_trip_nanoids() {
    let min_lens = [0, 4, 10, 18, 21, 50];
    verify_round_trip_for_collection(NANOID_SAMPLES, &min_lens);
}

#[test]
fn test_round_trip_domain_struct_commits() {
    let commits: Vec<Commit> = GIT_SHA1_SAMPLES
        .iter()
        .enumerate()
        .map(|(i, &h)| Commit {
            hash: h.to_string(),
            message: format!("Commit #{i}"),
        })
        .collect();

    let min_lens = [0, 4, 7, 10, 40, 80];
    verify_round_trip_for_collection(&commits, &min_lens);
}

#[test]
fn test_round_trip_dynamically_generated_uuids() {
    // Generate 40 unique deterministic UUID-formatted strings without external dependencies
    let uuids: Vec<String> = (0..40)
        .map(|i| {
            format!(
                "550e8400-e29b-41d4-a716-{:012x}",
                0x446655440000_u64 + i as u64
            )
        })
        .collect();
    let min_lens = [0, 4, 8, 12, 36, 100];
    verify_round_trip_for_collection(&uuids, &min_lens);
}

// ============================================================================
// 2. Ambiguity Guarantee
// ============================================================================

fn verify_ambiguity_disambiguation_guarantee<T: Identifiable + std::fmt::Debug + PartialEq>(
    candidates: &[T],
    ambiguous_prefix: &str,
) {
    let err_fn = find_by_id_prefix(candidates, ambiguous_prefix)
        .expect_err("Expected ambiguity error for ambiguous prefix from find_by_id_prefix");
    let err_ext = candidates
        .find_by_id_prefix(ambiguous_prefix)
        .expect_err("Expected ambiguity error for ambiguous prefix from IdSliceExt");
    assert_eq!(
        err_fn, err_ext,
        "Top-level and trait extension must return identical errors"
    );

    match err_fn {
        ResolveError::Ambiguous {
            prefix,
            candidates: colliding,
            required_len,
        } => {
            assert_eq!(prefix, ambiguous_prefix);
            assert!(
                colliding.len() >= 2,
                "Ambiguous error must report at least 2 colliding candidates, got {}",
                colliding.len()
            );

            for cand_id in &colliding {
                assert!(
                    cand_id.len() >= required_len,
                    "Candidate ID '{cand_id}' (len {}) must be >= required_len {required_len}",
                    cand_id.len()
                );
                assert!(
                    cand_id.is_char_boundary(required_len),
                    "required_len {required_len} must fall on valid UTF-8 boundary for '{cand_id}'"
                );

                let disambiguated = &cand_id[..required_len];
                let lookup_fn = find_by_id_prefix(candidates, disambiguated);
                let lookup_ext = candidates.find_by_id_prefix(disambiguated);

                assert_eq!(lookup_fn, lookup_ext);
                assert!(
                    lookup_fn.is_ok(),
                    "Disambiguated prefix '{disambiguated}' (len {required_len}) failed lookup: {:?}",
                    lookup_fn.err()
                );
                let resolved = lookup_fn.unwrap();
                assert_eq!(
                    resolved.id(),
                    cand_id.as_str(),
                    "Disambiguated prefix must resolve to exact candidate '{cand_id}'"
                );
            }
        }
        ResolveError::NotFound { .. } => {
            panic!("Expected Ambiguous error, but got NotFound for prefix '{ambiguous_prefix}'");
        }
    }
}

#[test]
fn test_ambiguity_guarantee_git_shas() {
    // Hashes sharing "e4d909c" (7 chars) and "e4d909" (6 chars)
    let candidates = [
        "e4d909c290d0fb1ca068ffaddf22cbd0add8b16a",
        "e4d909c390d0fb1ca068ffaddf22cbd0add8b16b",
        "e4d909a090d0fb1ca068ffaddf22cbd0add8b16c",
        "f111111111111111111111111111111111111111",
    ];

    // Ambiguous at "e4d909c": colliding are the two "e4d909c..." items, required_len should be 8
    verify_ambiguity_disambiguation_guarantee(&candidates, "e4d909c");

    // Ambiguous at "e4d909": colliding are 3 items, required_len should be 8
    verify_ambiguity_disambiguation_guarantee(&candidates, "e4d909");
}

#[test]
fn test_ambiguity_guarantee_uuids() {
    let candidates = [
        "550e8400-e29b-41d4-a716-446655440000",
        "550e8400-e29b-41d4-a716-446655440001",
        "550e8400-e29b-41d4-a716-446655440099",
        "770e8400-e29b-41d4-a716-446655440000",
    ];

    // Ambiguous at 8 chars
    verify_ambiguity_disambiguation_guarantee(&candidates, "550e8400");
    // Ambiguous at 34 chars: "550e8400-e29b-41d4-a716-4466554400"
    verify_ambiguity_disambiguation_guarantee(&candidates, "550e8400-e29b-41d4-a716-4466554400");
}

#[test]
fn test_ambiguity_guarantee_ulids() {
    let candidates = [
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "01ARZ3NDEKTSV4RRFFQ69G5FB0",
        "01ARZ3NDEKTSV4RRFFQ69G5FC1",
        "99ZZZZZZZZZZZZZZZZZZZZZZZZ",
    ];

    verify_ambiguity_disambiguation_guarantee(&candidates, "01ARZ");
    verify_ambiguity_disambiguation_guarantee(&candidates, "01ARZ3NDEKTSV4RRFFQ69G5F");
}

#[test]
fn test_ambiguity_guarantee_nanoids() {
    let candidates = [
        "V1StGXR8_Z5jdHi6B-myT",
        "V1StGXR8_Z5jdHi6B-myU",
        "V1StGXR8_Z5jdHi6B-myV",
        "ZZZZZZZZZZZZZZZZZZZZZ",
    ];

    verify_ambiguity_disambiguation_guarantee(&candidates, "V1StGXR8");
    verify_ambiguity_disambiguation_guarantee(&candidates, "V1StGXR8_Z5jdHi6B-my");
}

#[test]
fn test_ambiguity_guarantee_multibyte_korean() {
    let candidates = [
        "사용자-alpha-01",
        "사용자-alpha-02",
        "사용자-omega-03",
        "관리자-root-01",
    ];

    verify_ambiguity_disambiguation_guarantee(&candidates, "사용자");
    verify_ambiguity_disambiguation_guarantee(&candidates, "사용자-alpha");
}

#[test]
fn test_ambiguity_nested_prefix_candidate() {
    // When one candidate is a proper prefix of another
    let candidates = ["commit", "commit-feat-1", "branch-main"];

    let err = candidates
        .find_by_id_prefix("com")
        .expect_err("Should be ambiguous");
    match err {
        ResolveError::Ambiguous {
            candidates: colliding,
            required_len,
            ..
        } => {
            assert_eq!(
                colliding,
                vec!["commit".to_string(), "commit-feat-1".to_string()]
            );
            // Divergence between "commit" and "commit-feat-1" is 7 ("commit-")
            assert_eq!(required_len, 7);

            // "commit-feat-1"[..7] uniquely resolves to "commit-feat-1"
            let disambiguated = &colliding[1][..required_len];
            assert_eq!(disambiguated, "commit-");
            assert_eq!(
                candidates.find_by_id_prefix(disambiguated),
                Ok(&"commit-feat-1")
            );

            // Note: "commit" (length 6) is shorter than required_len (7) because
            // it is a proper prefix of "commit-feat-1", so no prefix can disambiguate
            // "commit" without also matching "commit-feat-1".
            assert!(colliding[0].len() < required_len);
        }
        _ => panic!("Expected Ambiguous error"),
    }
}

#[test]
fn test_ambiguity_guarantee_multibyte_emojis() {
    let candidates = [
        "🚀-worker-node-1",
        "🚀-worker-node-2",
        "🚀-manager-node-3",
        "📦-storage-node-4",
    ];

    verify_ambiguity_disambiguation_guarantee(&candidates, "🚀");
    verify_ambiguity_disambiguation_guarantee(&candidates, "🚀-worker");
}

#[test]
fn test_ambiguity_guarantee_domain_commits() {
    let commits = vec![
        Commit {
            hash: "commit-100-alpha".to_string(),
            message: "Alpha".to_string(),
        },
        Commit {
            hash: "commit-100-beta".to_string(),
            message: "Beta".to_string(),
        },
        Commit {
            hash: "commit-200-gamma".to_string(),
            message: "Gamma".to_string(),
        },
    ];

    verify_ambiguity_disambiguation_guarantee(&commits, "commit-100");
}

// ============================================================================
// 3. Lifetime Independence (AC-2)
// ============================================================================

#[test]
fn test_lifetime_independence_str_candidates_dropped() {
    let persistent_target = String::from("service-worker-production-01");
    let prefix_outlived: &str;

    {
        // candidates are created in this inner scope and dropped at its close
        let temporary_candidates = vec![
            String::from("service-worker-production-02"),
            String::from("service-worker-staging-01"),
            String::from("database-primary-01"),
        ];

        prefix_outlived = truncate_id_by_unique(&persistent_target, 10, &temporary_candidates);
        // temporary_candidates dropped here
    }

    assert_eq!(prefix_outlived, "service-worker-production-01");
    assert!(persistent_target.starts_with(prefix_outlived));
}

#[test]
fn test_lifetime_independence_domain_struct_candidates_dropped() {
    let persistent_commit = Commit {
        hash: "e4d909c290d0fb1ca068ffaddf22cbd0add8b16a".to_string(),
        message: "Keep this commit".to_string(),
    };

    let short_hash: &str;
    {
        let temporary_commits = vec![
            Commit {
                hash: "e4d909c390d0fb1ca068ffaddf22cbd0add8b16b".to_string(),
                message: "Temp 1".to_string(),
            },
            Commit {
                hash: "e4d909a090d0fb1ca068ffaddf22cbd0add8b16c".to_string(),
                message: "Temp 2".to_string(),
            },
        ];

        short_hash = persistent_commit.truncate_id_by_unique(7, &temporary_commits);
        // temporary_commits dropped here
    }

    // Divergence between ...c2 and ...c3 requires 8 chars ("e4d909c2")
    assert_eq!(short_hash, "e4d909c2");
    assert!(persistent_commit.hash.starts_with(short_hash));
}

#[test]
fn test_lifetime_independence_split_id_outliving_scope() {
    let target = String::from("user-session-987654321");
    let prefix: &str;
    let remainder: &str;

    {
        let candidate_scope = vec![
            String::from("user-session-987650000"),
            String::from("admin-session-111111111"),
        ];

        let (p, r) = target.split_id_by_unique(5, &candidate_scope);
        prefix = p;
        remainder = r;
        // candidate_scope dropped here
    }

    assert_eq!(prefix, "user-session-987654");
    assert_eq!(remainder, "321");
    assert_eq!(format!("{prefix}{remainder}"), target);
}

// ============================================================================
// 4. Multi-Byte UTF-8 Edge Cases (AC-3)
// ============================================================================

#[test]
fn test_multibyte_korean_exact_scenarios() {
    // Specifically test: "가나다1", "가나다2", "가나라"
    let candidates = ["가나다1", "가나다2", "가나라"];

    // "가나다1" vs "가나다2": LCP is "가나다" (3*3 = 9 bytes). Divergence requires 10 bytes ("가나다1").
    let target1 = "가나다1";
    let short1 = target1.truncate_id_by_unique(0, &candidates);
    assert_eq!(short1, "가나다1");
    assert_eq!(candidates.find_by_id_prefix(short1), Ok(&"가나다1"));

    // "가나라" vs ("가나다1", "가나다2"): LCP is "가나" (6 bytes). Divergence requires 6 + 3 = 9 bytes ("가나라").
    let target_ra = "가나라";
    let short_ra = target_ra.truncate_id_by_unique(0, &candidates);
    assert_eq!(short_ra, "가나라");
    assert_eq!(candidates.find_by_id_prefix(short_ra), Ok(&"가나라"));

    // Round-trip verification across all Korean items
    verify_round_trip_for_collection(&candidates, &[0, 1, 3, 6, 9, 10, 20]);
}

#[test]
fn test_multibyte_emoji_identifiers() {
    // Specifically test: "🚀-alpha", "🚀-beta"
    let candidates = ["🚀-alpha", "🚀-beta", "🦀-gamma"];

    // "🚀-alpha" vs "🚀-beta": LCP is "🚀-" (4 + 1 = 5 bytes).
    // Divergence: 5 + 1 ('a' / 'b') = 6 bytes ("🚀-a" / "🚀-b")
    let short_alpha = "🚀-alpha".truncate_id_by_unique(0, &candidates);
    assert_eq!(short_alpha, "🚀-a");
    assert_eq!(candidates.find_by_id_prefix(short_alpha), Ok(&"🚀-alpha"));

    let short_beta = "🚀-beta".truncate_id_by_unique(0, &candidates);
    assert_eq!(short_beta, "🚀-b");
    assert_eq!(candidates.find_by_id_prefix(short_beta), Ok(&"🚀-beta"));

    let short_gamma = "🦀-gamma".truncate_id_by_unique(0, &candidates);
    assert_eq!(short_gamma, "🦀");
    assert_eq!(candidates.find_by_id_prefix(short_gamma), Ok(&"🦀-gamma"));

    verify_round_trip_for_collection(&candidates, &[0, 1, 4, 5, 6, 12]);
}

#[test]
fn test_multibyte_2_3_4_byte_encodings() {
    // 1-byte: ASCII ('a', 'b')
    // 2-byte: 'é' (0xC3 0xA9), 'λ' (0xCE 0xBB), 'Ω' (0xCE 0xA9)
    // 3-byte: '가' (0xEA 0xB0 0x80), '日' (0xE6 0x97 0xA5)
    // 4-byte: '🚀' (0xF0 0x9F 0x9A 0x80), '🦀' (0xF0 0x9F 0xA6 0x80)

    let candidates = [
        "id-é-lambda-1",
        "id-é-lambda-2",
        "id-é-omega-3",
        "id-가-sun-4",
        "id-日-moon-5",
        "id-🚀-rocket-6",
        "id-🦀-crab-7",
    ];

    // Check byte lengths of characters
    assert_eq!('é'.len_utf8(), 2);
    assert_eq!('λ'.len_utf8(), 2);
    assert_eq!('Ω'.len_utf8(), 2);
    assert_eq!('가'.len_utf8(), 3);
    assert_eq!('日'.len_utf8(), 3);
    assert_eq!('🚀'.len_utf8(), 4);
    assert_eq!('🦀'.len_utf8(), 4);

    verify_round_trip_for_collection(&candidates, &[0, 2, 4, 7, 10, 15, 30]);

    // Test ambiguity on 2-byte char prefix
    let err = candidates
        .find_by_id_prefix("id-é-lambda")
        .expect_err("Should be ambiguous");
    match err {
        ResolveError::Ambiguous {
            candidates: colliding,
            required_len,
            ..
        } => {
            assert_eq!(colliding.len(), 2);
            for c in colliding {
                let disambiguated = &c[..required_len];
                assert_eq!(candidates.find_by_id_prefix(disambiguated), Ok(&c.as_str()));
            }
        }
        _ => panic!("Expected Ambiguous"),
    }
}

#[test]
fn test_multibyte_boundary_snapping_exact_before_after() {
    // Target: "🦀-가-é-x"
    // Layout:
    // '🦀' (4 bytes): [0..4]
    // '-'  (1 byte):  [4..5]
    // '가' (3 bytes): [5..8]
    // '-'  (1 byte):  [8..9]
    // 'é'  (2 bytes): [9..11]
    // '-'  (1 byte):  [11..12]
    // 'x'  (1 byte):  [12..13]
    let target = "🦀-가-é-x";
    assert_eq!(target.len(), 13);

    // Completely distinct candidate
    let candidates = ["other-identifier"];

    // Testing min_len snapping around 4-byte char '🦀'
    assert_eq!(truncate_id_by_unique(target, 0, &candidates), "🦀"); // divergence is 4
    assert_eq!(truncate_id_by_unique(target, 1, &candidates), "🦀"); // before boundary: snaps to 4
    assert_eq!(truncate_id_by_unique(target, 2, &candidates), "🦀"); // before boundary: snaps to 4
    assert_eq!(truncate_id_by_unique(target, 3, &candidates), "🦀"); // before boundary: snaps to 4
    assert_eq!(truncate_id_by_unique(target, 4, &candidates), "🦀"); // exact boundary: 4
    assert_eq!(truncate_id_by_unique(target, 5, &candidates), "🦀-"); // exact boundary after '-'

    // Testing min_len snapping around 3-byte char '가' [5..8]
    assert_eq!(truncate_id_by_unique(target, 6, &candidates), "🦀-가"); // inside '가': snaps to 8
    assert_eq!(truncate_id_by_unique(target, 7, &candidates), "🦀-가"); // inside '가': snaps to 8
    assert_eq!(truncate_id_by_unique(target, 8, &candidates), "🦀-가"); // exact boundary: 8
    assert_eq!(truncate_id_by_unique(target, 9, &candidates), "🦀-가-"); // exact boundary after '-'

    // Testing min_len snapping around 2-byte char 'é' [9..11]
    assert_eq!(truncate_id_by_unique(target, 10, &candidates), "🦀-가-é"); // inside 'é': snaps to 11
    assert_eq!(truncate_id_by_unique(target, 11, &candidates), "🦀-가-é"); // exact boundary: 11
    assert_eq!(truncate_id_by_unique(target, 12, &candidates), "🦀-가-é-"); // exact boundary
    assert_eq!(truncate_id_by_unique(target, 13, &candidates), "🦀-가-é-x"); // full length
    assert_eq!(truncate_id_by_unique(target, 100, &candidates), "🦀-가-é-x"); // clamp
}

#[test]
fn test_multibyte_split_boundary_preservation() {
    let target = "한글-🚀-태스크";
    let candidates = ["한글-🚀-테스트"];

    // "한글-🚀-태스크" vs "한글-🚀-테스트"
    // LCP: "한글-🚀-"
    // '한' (3) + '글' (3) + '-' (1) + '🚀' (4) + '-' (1) = 12 bytes
    // Next char '태' (3 bytes) diverges from '테' (3 bytes) -> divergence = 15 bytes ("한글-🚀-태")
    let (pfx, rem) = target.split_id_by_unique(0, &candidates);
    assert_eq!(pfx, "한글-🚀-태");
    assert_eq!(rem, "스크");
    assert_eq!(format!("{pfx}{rem}"), target);

    // Verify all splits across arbitrary min_lens produce valid UTF-8
    for min_len in 0..=30 {
        let (p, r) = target.split_id_by_unique(min_len, &candidates);
        assert_eq!(format!("{p}{r}"), target);
        assert!(std::str::from_utf8(p.as_bytes()).is_ok());
        assert!(std::str::from_utf8(r.as_bytes()).is_ok());
    }
}

// ============================================================================
// 5. Domain Struct Testing with impl_identifiable!
// ============================================================================

#[test]
fn test_domain_commit_trait_and_slice_extensions() {
    let commits = vec![
        Commit {
            hash: "c0ffee01a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7".to_string(),
            message: "feat: initial commit".to_string(),
        },
        Commit {
            hash: "c0ffee02a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7".to_string(),
            message: "fix: solve bug".to_string(),
        },
        Commit {
            hash: "deadbeef00112233445566778899aabbccddeeff".to_string(),
            message: "docs: update readme".to_string(),
        },
    ];

    // Slice extension method: find_by_id_prefix
    let found = commits
        .find_by_id_prefix("dead")
        .expect("Should resolve uniquely");
    assert_eq!(found.message, "docs: update readme");

    // Item extension method: truncate_id_by_unique
    let short_c0 = commits[0].truncate_id_by_unique(7, &commits);
    assert_eq!(short_c0, "c0ffee01");

    let short_c1 = commits[1].truncate_id_by_unique(7, &commits);
    assert_eq!(short_c1, "c0ffee02");

    let short_dead = commits[2].truncate_id_by_unique(4, &commits);
    assert_eq!(short_dead, "dead");

    // Item extension method: split_id_by_unique
    let (prefix, rem) = commits[0].split_id_by_unique(7, &commits);
    assert_eq!(prefix, "c0ffee01");
    assert_eq!(rem, "a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7");
    assert_eq!(format!("{prefix}{rem}"), commits[0].hash);

    // Extension method on slice round-trip
    assert_eq!(commits.find_by_id_prefix(short_c0).unwrap(), &commits[0]);
    assert_eq!(commits.find_by_id_prefix(short_c1).unwrap(), &commits[1]);
    assert_eq!(commits.find_by_id_prefix(short_dead).unwrap(), &commits[2]);
}

#[test]
fn test_domain_borrowed_lifetime_struct() {
    let deploys = [
        DeployRecord {
            id: "deploy-2026-prod-01",
            environment: "production",
        },
        DeployRecord {
            id: "deploy-2026-staging-01",
            environment: "staging",
        },
        DeployRecord {
            id: "deploy-2026-dev-01",
            environment: "development",
        },
    ];

    let found_prod = deploys.find_by_id_prefix("deploy-2026-p").unwrap();
    assert_eq!(found_prod.environment, "production");

    let short = deploys[0].truncate_id_by_unique(5, &deploys);
    assert_eq!(short, "deploy-2026-p");

    let (pfx, rem) = deploys[1].split_id_by_unique(5, &deploys);
    assert_eq!(pfx, "deploy-2026-s");
    assert_eq!(rem, "taging-01");
}

#[test]
fn test_domain_custom_identifiable_trait_impl() {
    let issues = [
        IssueTrackerItem {
            key: "PROJ-101".to_string(),
            priority: 1,
        },
        IssueTrackerItem {
            key: "PROJ-102".to_string(),
            priority: 2,
        },
        IssueTrackerItem {
            key: "INFRA-500".to_string(),
            priority: 3,
        },
    ];

    let item = issues.find_by_id_prefix("INFRA").unwrap();
    assert_eq!(item.priority, 3);

    let short = issues[0].truncate_id_by_unique(4, &issues);
    assert_eq!(short, "PROJ-101");
}

// ============================================================================
// 6. Edge Cases
// ============================================================================

#[test]
fn test_edge_case_single_candidate() {
    let candidates = ["solo-identifier-123"];

    // Target is candidate itself: candidate is skipped (cand_id == target_id)
    // Result is governed only by min_len clamped to target length
    assert_eq!(
        truncate_id_by_unique("solo-identifier-123", 0, &candidates),
        ""
    );
    assert_eq!(
        truncate_id_by_unique("solo-identifier-123", 4, &candidates),
        "solo"
    );
    assert_eq!(
        truncate_id_by_unique("solo-identifier-123", 50, &candidates),
        "solo-identifier-123"
    );

    // Target is different from sole candidate
    assert_eq!(
        truncate_id_by_unique("different-item", 0, &candidates),
        "d" // divergence is 1
    );
    assert_eq!(
        truncate_id_by_unique("different-item", 4, &candidates),
        "diff"
    );

    // Lookup on single candidate
    assert_eq!(
        candidates.find_by_id_prefix("solo"),
        Ok(&"solo-identifier-123")
    );
    assert_eq!(
        candidates.find_by_id_prefix("solo-identifier-123"),
        Ok(&"solo-identifier-123")
    );
    assert_eq!(
        candidates.find_by_id_prefix("nonexistent"),
        Err(ResolveError::NotFound {
            prefix: "nonexistent".to_string()
        })
    );
}

#[test]
fn test_edge_case_duplicate_identical_ids() {
    let candidates = ["duplicate-id", "duplicate-id", "unique-id"];

    // Looking up prefix "dup": both duplicates match -> Ambiguous error
    let err = candidates.find_by_id_prefix("dup").unwrap_err();
    assert_eq!(
        err,
        ResolveError::Ambiguous {
            prefix: "dup".to_string(),
            candidates: vec!["duplicate-id".to_string(), "duplicate-id".to_string()],
            required_len: 12,
        }
    );

    // Looking up exact duplicate id: still ambiguous because collection contains 2 identical items
    let err_exact = candidates.find_by_id_prefix("duplicate-id").unwrap_err();
    assert_eq!(
        err_exact,
        ResolveError::Ambiguous {
            prefix: "duplicate-id".to_string(),
            candidates: vec!["duplicate-id".to_string(), "duplicate-id".to_string()],
            required_len: 12,
        }
    );

    // Truncating a duplicate item skips identical candidates
    // Divergence with "unique-id" is 1 ('d' vs 'u')
    let target = "duplicate-id";
    assert_eq!(truncate_id_by_unique(target, 0, &candidates), "d");
    assert_eq!(truncate_id_by_unique(target, 4, &candidates), "dupl");
}

#[test]
fn test_edge_case_prefix_longer_than_any_id() {
    let candidates = ["short", "tiny", "mini"];
    let long_prefix = "short-but-way-too-long-to-match-any-candidate";

    let err = candidates.find_by_id_prefix(long_prefix).unwrap_err();
    assert_eq!(
        err,
        ResolveError::NotFound {
            prefix: long_prefix.to_string(),
        }
    );
}

#[test]
fn test_edge_case_empty_candidates() {
    let empty: [&str; 0] = [];

    // Prefix lookup on empty list -> NotFound
    let err = empty.find_by_id_prefix("anything").unwrap_err();
    assert_eq!(
        err,
        ResolveError::NotFound {
            prefix: "anything".to_string()
        }
    );

    let err_empty_prefix = empty.find_by_id_prefix("").unwrap_err();
    assert_eq!(
        err_empty_prefix,
        ResolveError::NotFound {
            prefix: "".to_string()
        }
    );

    // Truncate against empty candidate list returns min_len clamped
    let target = "sample-identifier";
    assert_eq!(truncate_id_by_unique(target, 0, &empty), "");
    assert_eq!(truncate_id_by_unique(target, 6, &empty), "sample");
    assert_eq!(
        truncate_id_by_unique(target, 50, &empty),
        "sample-identifier"
    );

    let (pfx, rem) = split_id_by_unique(target, 6, &empty);
    assert_eq!(pfx, "sample");
    assert_eq!(rem, "-identifier");
}

#[test]
fn test_edge_case_min_len_zero() {
    let candidates = ["alpha-one", "beta-two", "gamma-three"];

    // min_len = 0 returns minimal 1-char divergence
    assert_eq!(truncate_id_by_unique("alpha-one", 0, &candidates), "a");
    assert_eq!(truncate_id_by_unique("beta-two", 0, &candidates), "b");
    assert_eq!(truncate_id_by_unique("gamma-three", 0, &candidates), "g");

    // Reversible via find_by_id_prefix
    assert_eq!(candidates.find_by_id_prefix("a"), Ok(&"alpha-one"));
    assert_eq!(candidates.find_by_id_prefix("b"), Ok(&"beta-two"));
    assert_eq!(candidates.find_by_id_prefix("g"), Ok(&"gamma-three"));
}

#[test]
fn test_edge_case_min_len_larger_than_id() {
    let candidates = ["alpha", "beta"];

    // min_len = 500 on 5-character string
    let target = "alpha";
    let truncated = truncate_id_by_unique(target, 500, &candidates);
    assert_eq!(truncated, "alpha");

    let (prefix, remainder) = split_id_by_unique(target, 500, &candidates);
    assert_eq!(prefix, "alpha");
    assert_eq!(remainder, "");
    assert_eq!(format!("{prefix}{remainder}"), target);

    // Multi-byte target with large min_len
    let mb_target = "한글태스크";
    let mb_candidates = ["한글프로젝트"];
    let mb_truncated = truncate_id_by_unique(mb_target, 9999, &mb_candidates);
    assert_eq!(mb_truncated, "한글태스크");
    let (mb_p, mb_r) = split_id_by_unique(mb_target, 9999, &mb_candidates);
    assert_eq!(mb_p, "한글태스크");
    assert_eq!(mb_r, "");
}
