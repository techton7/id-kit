//! Verification of examples published in README.md.

use id_kit::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Task {
    id: String,
    title: String,
}

#[test]
fn test_readme_quickstart() {
    let tasks = vec![
        Task {
            id: "task-deploy-web".to_string(),
            title: "Deploy web service".to_string(),
        },
        Task {
            id: "task-deploy-db".to_string(),
            title: "Deploy database".to_string(),
        },
        Task {
            id: "task-test".to_string(),
            title: "Run tests".to_string(),
        },
    ];

    // 1. Prefix lookup: ambiguous prefix returns ResolveError::Ambiguous
    match tasks.find_by_id_prefix("task-d", |t| &t.id) {
        Err(ResolveError::Ambiguous {
            prefix,
            candidates,
            required_len,
        }) => {
            assert_eq!(prefix, "task-d");
            assert_eq!(candidates.len(), 2);
            assert_eq!(required_len, 13); // Disambiguation requires "task-deploy-w"
        }
        _ => panic!("expected Ambiguous"),
    }

    // Lookup with disambiguated prefix resolves uniquely
    let task = tasks
        .find_by_id_prefix("task-deploy-w", |t| &t.id)
        .expect("should find task-deploy-web");
    assert_eq!(task.title, "Deploy web service");

    // 2. Minimal unique prefix truncation (min_len: 5)
    let unique_prefix = task.id.truncate_id_by_unique(5, &tasks, |t| &t.id);
    assert_eq!(unique_prefix, "task-deploy-w");

    // 3. Minimal unique prefix splitting
    let (prefix, remainder) = task.id.split_id_by_unique(5, &tasks, |t| &t.id);
    assert_eq!(prefix, "task-deploy-w");
    assert_eq!(remainder, "eb");
}

#[test]
fn test_readme_operation1_prefix_lookup() {
    let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

    // Trait extension method on slice: unique prefix matches
    let matched = candidates
        .find_by_id_prefix("item-alpi", |s| *s)
        .expect("should find item-alpine");
    assert_eq!(*matched, "item-alpine");

    // Top-level function
    let matched_fn = id_kit::find_by_id_prefix(&candidates, "item-alpi", |s| *s)
        .expect("should find item-alpine");
    assert_eq!(*matched_fn, "item-alpine");

    // Error handling: NotFound
    let not_found = candidates.find_by_id_prefix("unknown", |s| *s);
    assert_eq!(
        not_found,
        Err(ResolveError::NotFound {
            prefix: "unknown".to_string(),
        })
    );

    // Error handling: Ambiguous
    match candidates.find_by_id_prefix("item-alp", |s| *s) {
        Err(ResolveError::Ambiguous {
            prefix,
            candidates,
            required_len,
        }) => {
            assert_eq!(prefix, "item-alp");
            assert_eq!(candidates, vec!["item-alpha", "item-alpine"]);
            assert_eq!(required_len, 9); // "item-alph" vs "item-alpi"
        }
        _ => panic!("expected Ambiguous"),
    }
}

#[test]
fn test_readme_operation2_truncation_and_splitting() {
    let target = "commit-a1b2c3";
    let candidates = vec!["commit-a1b2d4", "commit-a1b999", "commit-f00000"];

    // Truncate to minimal unique prefix (with min_len = 4)
    let prefix = target.truncate_id_by_unique(4, &candidates, |s| *s);
    assert_eq!(prefix, "commit-a1b2c");

    // Split into unique prefix and remaining suffix
    let (unique_part, remainder) = target.split_id_by_unique(4, &candidates, |s| *s);
    assert_eq!(unique_part, "commit-a1b2c");
    assert_eq!(remainder, "3");

    // Top-level functions
    let fn_prefix = id_kit::truncate_id_by_unique(target, 4, &candidates, |s| *s);
    assert_eq!(fn_prefix, "commit-a1b2c");

    let (fn_unique_part, fn_remainder) = id_kit::split_id_by_unique(target, 4, &candidates, |s| *s);
    assert_eq!(fn_unique_part, "commit-a1b2c");
    assert_eq!(fn_remainder, "3");

    // Self-equality exclusion
    let self_target = "task-42";
    let self_candidates = vec!["task-42", "task-40", "task-50"];
    let self_prefix = self_target.truncate_id_by_unique(2, &self_candidates, |s| *s);
    assert_eq!(self_prefix, "task-42");

    // Lifetime independence
    let owned_target = String::from("service-auth-node-1");
    let prefix_outlived: String;
    {
        let temporary_candidates = vec![
            String::from("service-auth-node-2"),
            String::from("service-cache-node-1"),
        ];
        prefix_outlived =
            id_kit::truncate_id_by_unique(&owned_target, 4, &temporary_candidates, |s| s);
    }
    assert_eq!(prefix_outlived, "service-auth-node-1");
}

#[test]
fn test_readme_foreign_dtos_and_value_types() {
    // Simulated foreign value-type ID (e.g. from uuid or ulid crate)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Uuid([u8; 16]);

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for byte in &self.0 {
                write!(f, "{:02x}", byte)?;
            }
            Ok(())
        }
    }

    // Simulated foreign DTO struct
    #[derive(Debug, Clone)]
    struct NodeDto {
        id: Uuid,
        name: String,
    }

    let nodes = vec![
        NodeDto {
            id: Uuid([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0, 0, 0, 0, 0, 0, 0, 1,
            ]),
            name: "gateway-1".to_string(),
        },
        NodeDto {
            id: Uuid([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xee, 0, 0, 0, 0, 0, 0, 0, 2,
            ]),
            name: "gateway-2".to_string(),
        },
        NodeDto {
            id: Uuid([
                0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0, 0, 0, 0, 0, 0, 0, 3,
            ]),
            name: "worker-1".to_string(),
        },
    ];

    // 1. Prefix lookup with closure generating temporary String
    let found = nodes
        .find_by_id_prefix("9988", |n| n.id.to_string())
        .expect("should find worker-1");
    assert_eq!(found.name, "worker-1");

    // 2. Safe inline chaining on temporary target: node.id.to_string()
    let unique_prefix = nodes[0]
        .id
        .to_string()
        .truncate_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(unique_prefix, "0123456789abcdef");

    // 3. Safe inline splitting on temporary target
    let (prefix, remainder) = nodes[0]
        .id
        .to_string()
        .split_id_by_unique(8, &nodes, |n| n.id.to_string());
    assert_eq!(prefix, "0123456789abcdef");
    assert_eq!(remainder, "0000000000000001");
}

#[test]
fn test_readme_utf8_multibyte() {
    let target = "사용자-001";
    let candidates = vec!["사용자-002", "관리자-001"];

    let unique = target.truncate_id_by_unique(1, &candidates, |s| *s);
    assert_eq!(unique, "사용자-001");

    let single_candidate = vec!["관리자-001"];
    assert_eq!(
        target.truncate_id_by_unique(2, &single_candidate, |s| *s),
        "사"
    );
}
