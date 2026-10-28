//! Verification of examples published in README.md.

use id_kit::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Task {
    id: String,
    title: String,
}

impl_identifiable!(Task, id);

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
    match tasks.find_by_id_prefix("task-d") {
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
    let task = tasks.find_by_id_prefix("task-deploy-w").unwrap();
    assert_eq!(task.title, "Deploy web service");

    // 2. Minimal unique prefix truncation (min_len: 5)
    let unique_prefix = task.truncate_id_by_unique(5, &tasks);
    assert_eq!(unique_prefix, "task-deploy-w");

    // 3. Minimal unique prefix splitting
    let (prefix, remainder) = task.split_id_by_unique(5, &tasks);
    assert_eq!(prefix, "task-deploy-w");
    assert_eq!(remainder, "eb");
}

#[test]
fn test_readme_operation1_prefix_lookup() {
    let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

    // Trait extension method on slice: unique prefix matches
    let matched = candidates.find_by_id_prefix("item-alpi").unwrap();
    assert_eq!(*matched, "item-alpine");

    // Top-level function
    let matched_fn = id_kit::find_by_id_prefix(&candidates, "item-alpi").unwrap();
    assert_eq!(*matched_fn, "item-alpine");

    // Error handling: NotFound
    let not_found = candidates.find_by_id_prefix("unknown");
    assert_eq!(
        not_found,
        Err(ResolveError::NotFound {
            prefix: "unknown".to_string(),
        })
    );

    // Error handling: Ambiguous
    match candidates.find_by_id_prefix("item-alp") {
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
    let prefix = target.truncate_id_by_unique(4, &candidates);
    assert_eq!(prefix, "commit-a1b2c");

    // Split into unique prefix and remaining suffix
    let (unique_part, remainder) = target.split_id_by_unique(4, &candidates);
    assert_eq!(unique_part, "commit-a1b2c");
    assert_eq!(remainder, "3");

    // Top-level functions
    let fn_prefix = id_kit::truncate_id_by_unique(&target, 4, &candidates);
    assert_eq!(fn_prefix, "commit-a1b2c");

    let (fn_unique_part, fn_remainder) = id_kit::split_id_by_unique(&target, 4, &candidates);
    assert_eq!(fn_unique_part, "commit-a1b2c");
    assert_eq!(fn_remainder, "3");

    // Self-equality exclusion
    let self_target = "task-42";
    let self_candidates = vec!["task-42", "task-40", "task-50"];
    let self_prefix = self_target.truncate_id_by_unique(2, &self_candidates);
    assert_eq!(self_prefix, "task-42");

    // Lifetime guarantees
    let owned_target = String::from("service-auth-node-1");
    let borrowed_prefix: &str;
    {
        let temporary_candidates = vec![
            String::from("service-auth-node-2"),
            String::from("service-cache-node-1"),
        ];
        borrowed_prefix = id_kit::truncate_id_by_unique(&owned_target, 4, &temporary_candidates);
    }
    assert_eq!(borrowed_prefix, "service-auth-node-1");
}

#[test]
fn test_readme_macro_variants() {
    struct RefTask<'a> {
        id: &'a str,
    }
    impl_identifiable!(RefTask<'a>, id);

    struct AnonTask<'a> {
        id: &'a str,
    }
    impl_identifiable!(AnonTask<'_>, id);

    struct GenericTask<T> {
        id: String,
        _data: T,
    }
    impl_identifiable!(GenericTask<T>, id);

    struct TupleTask(String, usize);
    impl_identifiable!(TupleTask, 0);

    let t1 = RefTask { id: "ref-1" };
    assert_eq!(t1.id(), "ref-1");

    let t2 = AnonTask { id: "anon-2" };
    assert_eq!(t2.id(), "anon-2");

    let t3 = GenericTask {
        id: "gen-3".to_string(),
        _data: 42,
    };
    assert_eq!(t3.id(), "gen-3");

    let t4 = TupleTask("tup-4".to_string(), 100);
    assert_eq!(t4.id(), "tup-4");
    assert_eq!(t4.1, 100);
}

#[test]
fn test_readme_utf8_multibyte() {
    let target = "사용자-001";
    let candidates = vec!["사용자-002", "관리자-001"];

    let unique = target.truncate_id_by_unique(1, &candidates);
    assert_eq!(unique, "사용자-001");

    let single_candidate = vec!["관리자-001"];
    assert_eq!(target.truncate_id_by_unique(2, &single_candidate), "사");
}
