//! Timerit consumer simulation integration tests.
//!
//! Validates `id-kit` integration under `timerit` consumer application constraints:
//! 1. Foreign DTO struct (`NodeResponse`) and foreign value-type (`SimulatedUuid`).
//! 2. Zero orphan-rule friction: NO traits from `id_kit` are implemented on foreign types.
//! 3. Closure-based key extraction returning temporary owned `String` (`|n| n.id.to_string()`).
//! 4. Temporary target chaining without `E0716` ("temporary value dropped while borrowed").
//! 5. Owned `String` returns for truncation and splitting, allowing lifetime independence.
//! 6. Ambiguity detection returning candidates and exact `required_len` for disambiguation.
//! 7. Zero candidates and single candidate boundary behaviors.

use id_kit::{
    find_by_id_prefix, split_id_by_unique, truncate_id_by_unique, IdSliceExt, IdUniqueExt,
    ResolveError,
};

// ============================================================================
// Foreign Types (Simulating timerit's data models)
// ============================================================================

/// Foreign value-type simulating a 128-bit UUID (e.g. from the `uuid` crate).
///
/// Implements `Display` to format as 32 lowercase hex characters.
/// Crucially: does NOT implement any trait from `id_kit` (e.g., `AsRef<str>`, `IdUniqueExt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimulatedUuid([u8; 16]);

impl SimulatedUuid {
    /// Creates a simulated UUID from raw 16 bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Parses a 32-character hexadecimal string into a `SimulatedUuid`.
    ///
    /// # Panics
    /// Panics if the hex string is not exactly 32 valid hex characters.
    #[must_use]
    pub fn parse_hex(s: &str) -> Self {
        assert_eq!(s.len(), 32, "SimulatedUuid requires 32 hex chars");
        let mut bytes = [0u8; 16];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let pair = std::str::from_utf8(chunk).expect("valid utf-8 chunk");
            bytes[i] = u8::from_str_radix(pair, 16).expect("valid hex byte");
        }
        Self(bytes)
    }
}

impl std::fmt::Display for SimulatedUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Foreign DTO struct returned by timerit's remote or local node service.
///
/// Crucially: does NOT implement any trait from `id_kit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeResponse {
    pub id: SimulatedUuid,
    pub name: String,
    pub status: String,
}

impl NodeResponse {
    #[must_use]
    pub fn new(hex_id: &str, name: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: SimulatedUuid::parse_hex(hex_id),
            name: name.into(),
            status: status.into(),
        }
    }
}

// ============================================================================
// Fixtures
// ============================================================================

/// Creates a representative set of timerit `NodeResponse` fixtures.
///
/// Nodes 0, 1, and 2 share the initial prefix `"01234567"`.
/// - Node 0 and Node 1 share prefix up to index 15 (`"0123456789abcde"`), diverging at index 15 ('f' vs 'e').
/// - Node 2 diverges from Node 0 and 1 at index 8 ('8' vs 'a').
/// - Node 3 has completely different prefix `"99999999..."`.
fn setup_sample_nodes() -> Vec<NodeResponse> {
    vec![
        NodeResponse::new(
            "0123456789abcdef0123456789abcdef",
            "primary-gateway",
            "active",
        ),
        NodeResponse::new(
            "0123456789abcdee0000000000000000",
            "secondary-gateway",
            "standby",
        ),
        NodeResponse::new(
            "01234567aaaaaaaa1111111111111111",
            "worker-agent-1",
            "active",
        ),
        NodeResponse::new(
            "99999999888877776666555544443333",
            "database-coordinator",
            "idle",
        ),
    ]
}

// ============================================================================
// Test Cases
// ============================================================================

#[test]
fn test_prefix_lookup_with_closure_returning_temporary_string() {
    let nodes = setup_sample_nodes();

    // 1. Slice extension method: nodes.find_by_id_prefix(&prefix, |n| n.id.to_string())
    let prefix = "9999".to_string();
    let found_ext = nodes
        .find_by_id_prefix(&prefix, |n| n.id.to_string())
        .expect("should find database-coordinator");
    assert_eq!(found_ext.name, "database-coordinator");
    assert_eq!(found_ext.status, "idle");

    // 2. Top-level function: find_by_id_prefix(&nodes, &prefix, |n| n.id.to_string())
    let found_fn = find_by_id_prefix(&nodes, &prefix, |n| n.id.to_string())
        .expect("should find database-coordinator via top-level function");
    assert_eq!(found_fn.name, "database-coordinator");

    // 3. Unique match among shared-prefix nodes by extending prefix length
    let node0_unique_prefix = "0123456789abcdef";
    let found_node0 = nodes
        .find_by_id_prefix(node0_unique_prefix, |n| n.id.to_string())
        .expect("should resolve node 0 uniquely");
    assert_eq!(found_node0.name, "primary-gateway");

    let node1_unique_prefix = "0123456789abcdee";
    let found_node1 = nodes
        .find_by_id_prefix(node1_unique_prefix, |n| n.id.to_string())
        .expect("should resolve node 1 uniquely");
    assert_eq!(found_node1.name, "secondary-gateway");

    let node2_unique_prefix = "01234567a";
    let found_node2 = nodes
        .find_by_id_prefix(node2_unique_prefix, |n| n.id.to_string())
        .expect("should resolve node 2 uniquely");
    assert_eq!(found_node2.name, "worker-agent-1");
}

#[test]
fn test_inline_chaining_with_temporary_target() {
    let nodes = setup_sample_nodes();
    let node = &nodes[0];

    // Chaining directly on temporary String from node.id.to_string()
    // Verifies NO E0716 temporary dropped while borrowed error occurs.
    let unique_via_to_string: String = node
        .id
        .to_string()
        .truncate_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(unique_via_to_string, "0123456789abcdef");

    // Chaining directly on temporary String from format!("{}", node.id)
    let unique_via_format: String =
        format!("{}", node.id).truncate_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(unique_via_format, "0123456789abcdef");

    // Chaining split_id_by_unique directly on temporary String
    let (prefix_via_to_string, rem_via_to_string): (String, String) = node
        .id
        .to_string()
        .split_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(prefix_via_to_string, "0123456789abcdef");
    assert_eq!(rem_via_to_string, "0123456789abcdef");

    let (prefix_via_format, rem_via_format): (String, String) =
        format!("{}", node.id).split_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(prefix_via_format, "0123456789abcdef");
    assert_eq!(rem_via_format, "0123456789abcdef");
}

#[test]
fn test_owned_string_returns_for_truncate_and_split() {
    let nodes = setup_sample_nodes();
    let node = &nodes[0];

    // 1. Owned String return for truncate_id_by_unique
    let short: String = truncate_id_by_unique(node.id.to_string(), 8, &nodes, |n| n.id.to_string());
    assert_eq!(short, "0123456789abcdef");

    // 2. Owned (String, String) return for split_id_by_unique
    let (pfx, rem): (String, String) =
        split_id_by_unique(node.id.to_string(), 8, &nodes, |n| n.id.to_string());
    assert_eq!(pfx, "0123456789abcdef");
    assert_eq!(rem, "0123456789abcdef");

    // 3. Round-trip concatenation equality
    assert_eq!(format!("{pfx}{rem}"), node.id.to_string());

    // 4. Verify lifetime independence: returned Strings outlive candidate list scope
    let target_uuid = node.id;
    let (isolated_short, isolated_pfx, isolated_rem) = {
        let temporary_nodes = setup_sample_nodes();
        let s = truncate_id_by_unique(target_uuid.to_string(), 8, &temporary_nodes, |n| {
            n.id.to_string()
        });
        let (p, r) = split_id_by_unique(target_uuid.to_string(), 8, &temporary_nodes, |n| {
            n.id.to_string()
        });
        (s, p, r)
    }; // temporary_nodes dropped here

    assert_eq!(isolated_short, "0123456789abcdef");
    assert_eq!(isolated_pfx, "0123456789abcdef");
    assert_eq!(isolated_rem, "0123456789abcdef");
    assert_eq!(
        format!("{isolated_pfx}{isolated_rem}"),
        target_uuid.to_string()
    );
}

#[test]
fn test_ambiguity_handling() {
    let nodes = setup_sample_nodes();

    // Query with ambiguous prefix shared by nodes 0, 1, and 2
    let ambiguous_prefix = "01234567";
    let err = nodes
        .find_by_id_prefix(ambiguous_prefix, |n| n.id.to_string())
        .expect_err("should return Ambiguous error for shared prefix");

    match err {
        ResolveError::Ambiguous {
            prefix,
            candidates,
            required_len,
        } => {
            assert_eq!(prefix, ambiguous_prefix);
            assert_eq!(candidates.len(), 3);
            assert!(candidates.contains(&nodes[0].id.to_string()));
            assert!(candidates.contains(&nodes[1].id.to_string()));
            assert!(candidates.contains(&nodes[2].id.to_string()));
            // Node 0 and Node 1 share 15 chars ("0123456789abcde").
            // To disambiguate them, at least 16 characters are required.
            assert_eq!(required_len, 16);

            // Verify that using required_len prefix resolves uniquely
            let node0_id = nodes[0].id.to_string();
            let disambiguated_prefix = &node0_id[..required_len];
            let resolved = nodes
                .find_by_id_prefix(disambiguated_prefix, |n| n.id.to_string())
                .expect("prefix of required_len should resolve unambiguously");
            assert_eq!(resolved.name, nodes[0].name);
        }
        ResolveError::NotFound { .. } => {
            panic!("expected ResolveError::Ambiguous, got NotFound");
        }
    }
}

#[test]
fn test_zero_candidates_behavior() {
    let empty_nodes: Vec<NodeResponse> = Vec::new();
    let query_prefix = "01234567";

    // 1. Slice prefix lookup on empty slice
    let slice_result = empty_nodes.find_by_id_prefix(query_prefix, |n| n.id.to_string());
    assert_eq!(
        slice_result,
        Err(ResolveError::NotFound {
            prefix: query_prefix.to_string(),
        })
    );

    // 2. Top-level prefix lookup on empty slice
    let fn_result = find_by_id_prefix(&empty_nodes, query_prefix, |n| n.id.to_string());
    assert_eq!(
        fn_result,
        Err(ResolveError::NotFound {
            prefix: query_prefix.to_string(),
        })
    );

    // 3. Truncation against empty candidates defaults to clamped min_len
    let sample_uuid = SimulatedUuid::parse_hex("0123456789abcdef0123456789abcdef");
    let truncated = truncate_id_by_unique(sample_uuid.to_string(), 8, &empty_nodes, |n| {
        n.id.to_string()
    });
    assert_eq!(truncated, "01234567");

    // 4. Splitting against empty candidates defaults to clamped min_len split
    let (prefix, remainder) = split_id_by_unique(sample_uuid.to_string(), 8, &empty_nodes, |n| {
        n.id.to_string()
    });
    assert_eq!(prefix, "01234567");
    assert_eq!(remainder, "89abcdef0123456789abcdef");
    assert_eq!(format!("{prefix}{remainder}"), sample_uuid.to_string());
}

#[test]
fn test_single_candidate_behavior() {
    let single_node = NodeResponse::new("0123456789abcdef0123456789abcdef", "solo-node", "running");
    let single_list = vec![single_node.clone()];

    // 1. Matching prefix on single candidate returns Ok
    let found = single_list
        .find_by_id_prefix("0123", |n| n.id.to_string())
        .expect("should match lone candidate");
    assert_eq!(found.name, "solo-node");

    // 2. Non-matching prefix on single candidate returns NotFound
    let not_found = single_list.find_by_id_prefix("ffff", |n| n.id.to_string());
    assert_eq!(
        not_found,
        Err(ResolveError::NotFound {
            prefix: "ffff".to_string(),
        })
    );

    // 3. Truncating candidate against list containing only itself (self-exclusion)
    // Candidate identical to target is excluded; falls back to min_len.
    let truncated_self = single_node
        .id
        .to_string()
        .truncate_id_by_unique(8, &single_list, |n| n.id.to_string());
    assert_eq!(truncated_self, "01234567");

    // 4. Truncating target against a single different candidate in the list
    let other_target = SimulatedUuid::parse_hex("0123456789abcdee0000000000000000");
    // other_target conflicts with single_node up to index 15 ('e' vs 'f')
    let truncated_other = other_target
        .to_string()
        .truncate_id_by_unique(8, &single_list, |n| n.id.to_string());
    assert_eq!(truncated_other, "0123456789abcdee");
}

#[test]
fn test_not_found_with_multiple_non_matching_candidates() {
    let nodes = setup_sample_nodes();
    let query_prefix = "nonexistent";

    let err = nodes
        .find_by_id_prefix(query_prefix, |n| n.id.to_string())
        .expect_err("should return NotFound for nonexistent prefix");

    assert_eq!(
        err,
        ResolveError::NotFound {
            prefix: query_prefix.to_string(),
        }
    );
}

#[test]
fn test_round_trip_recovery_for_all_nodes() {
    let nodes = setup_sample_nodes();

    for target in &nodes {
        let unique_prefix = target
            .id
            .to_string()
            .truncate_id_by_unique(8, &nodes, |n| n.id.to_string());

        let resolved = nodes
            .find_by_id_prefix(&unique_prefix, |n| n.id.to_string())
            .expect("unique prefix must resolve back to target node");

        assert_eq!(resolved.id, target.id);
        assert_eq!(resolved.name, target.name);
    }
}
