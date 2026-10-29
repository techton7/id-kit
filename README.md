# id-kit

[![Crates.io](https://img.shields.io/crates/v/id-kit.svg)](https://crates.io/crates/id-kit)
[![Documentation](https://docs.rs/id-kit/badge.svg)](https://docs.rs/id-kit)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Zero-dependency, `#![forbid(unsafe_code)]` Rust toolkit specializing strictly in:

1. Prefix-based ID lookup (`find_by_id_prefix` / `candidates.find_by_id_prefix(...)`)
2. Minimal unique prefix truncation and splitting (`truncate_id_by_unique`, `split_id_by_unique` / `target.truncate_id_by_unique(...)`, `target.split_id_by_unique(...)`)

## Key Properties

- **Zero External Dependencies**: Implemented using standard library primitives (`std` only).
- **Closure-Based Key Extraction**: Extractor closures (`key_fn: Fn(&'a T) -> K` where `K: AsRef<str>`) allow searching and truncating collections of domain structs and foreign DTOs directly without implementing traits or adapting data structures.
- **Owned String Returns & Safe Inline Chaining**: `truncate_id_by_unique` returns `String` and `split_id_by_unique` returns `(String, String)`. Because methods accept `target: impl AsRef<str>`, temporary expressions like `item.id.to_string().truncate_id_by_unique(...)` chain inline without lifetime or borrow conflicts (`E0716`).
- **Exact Matching**: Prefix lookups use case-sensitive exact matching (`starts_with`) without implicit transformations or trimming.
- **UTF-8 Safety**: Length calculations operate on byte offsets while enforcing valid Unicode scalar value boundaries via character boundary ceiling (`ceil_char_boundary`). Slicing panics on multi-byte codepoints are prevented.
- **Memory Safety**: Enforced with `#![forbid(unsafe_code)]`.
- **Lifetime Independence**: Returned prefixes and remainders are owned strings independent of the candidate collection lifetime.

---

## Installation

Add the dependency to `Cargo.toml`:

```toml
[dependencies]
id-kit = "0.2.0"
```

---

## Quickstart

Import the prelude to bring core traits and functions into scope:

```rust
use id_kit::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Task {
    id: String,
    title: String,
}

fn main() -> Result<(), ResolveError> {
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
        Err(ResolveError::Ambiguous { prefix, candidates, required_len }) => {
            assert_eq!(prefix, "task-d");
            assert_eq!(candidates.len(), 2);
            assert_eq!(required_len, 13); // Disambiguation requires "task-deploy-w"
        }
        _ => unreachable!(),
    }

    // Lookup with disambiguated prefix resolves uniquely
    let task = tasks.find_by_id_prefix("task-deploy-w", |t| &t.id)?;
    assert_eq!(task.title, "Deploy web service");

    // 2. Minimal unique prefix truncation (min_len: 5)
    let unique_prefix = task.id.truncate_id_by_unique(5, &tasks, |t| &t.id);
    assert_eq!(unique_prefix, "task-deploy-w");

    // 3. Minimal unique prefix splitting
    let (prefix, remainder) = task.id.split_id_by_unique(5, &tasks, |t| &t.id);
    assert_eq!(prefix, "task-deploy-w");
    assert_eq!(remainder, "eb");

    Ok(())
}
```

---

## Core Operations

### Operation 1: Prefix Lookup

Prefix lookup resolves a single item from a candidate collection whose identifier begins with the requested prefix.

#### Trait Extension Method

Available on slices `[T]` via [`IdSliceExt`]:

```rust
pub trait IdSliceExt<T> {
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
```

```rust
use id_kit::prelude::*;

let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

let matched = candidates.find_by_id_prefix("item-alpi", |s| *s)?;
assert_eq!(*matched, "item-alpine");
# Ok::<(), ResolveError>(())
```

#### Top-Level Function

```rust
pub fn find_by_id_prefix<'a, T, K, F>(
    candidates: &'a [T],
    prefix: &str,
    key_fn: F,
) -> Result<&'a T, ResolveError>
where
    F: Fn(&'a T) -> K,
    K: AsRef<str>;
```

```rust
use id_kit::find_by_id_prefix;

let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

let matched = find_by_id_prefix(&candidates, "item-alpi", |s| *s)?;
assert_eq!(*matched, "item-alpine");
# Ok::<(), id_kit::ResolveError>(())
```

#### Error Handling

Prefix lookup returns `Result<&T, ResolveError>`:

- `ResolveError::NotFound { prefix }`:
  Returned when zero candidates match the requested prefix.

- `ResolveError::Ambiguous { prefix, candidates, required_len }`:
  Returned when two or more candidates match the prefix.
  - `candidates`: Vector of matching identifier strings.
  - `required_len`: Minimum byte prefix length required to disambiguate the candidates, computed using pairwise Longest Common Prefix (LCP) analysis.

```rust
use id_kit::prelude::*;

let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

match candidates.find_by_id_prefix("item-alp", |s| *s) {
    Err(ResolveError::Ambiguous { prefix, candidates, required_len }) => {
        assert_eq!(prefix, "item-alp");
        assert_eq!(candidates, vec!["item-alpha", "item-alpine"]);
        assert_eq!(required_len, 9); // "item-alph" vs "item-alpi"
    }
    _ => unreachable!(),
}
```

---

### Operation 2: Minimal Unique Prefix Truncation & Splitting

Computes the shortest prefix of a target identifier that distinguishes it from all other candidates in a collection.

#### Trait Extension Methods

Available on any string-like type via [`IdUniqueExt`] (blanket implementation for all `S: AsRef<str> + ?Sized`):

```rust
pub trait IdUniqueExt {
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
```

```rust
use id_kit::prelude::*;

let target = "commit-a1b2c3";
let candidates = vec!["commit-a1b2d4", "commit-a1b999", "commit-f00000"];

// Truncate to minimal unique prefix (with min_len = 4)
let prefix = target.truncate_id_by_unique(4, &candidates, |s| *s);
assert_eq!(prefix, "commit-a1b2c");

// Split into unique prefix and remaining suffix
let (unique_part, remainder) = target.split_id_by_unique(4, &candidates, |s| *s);
assert_eq!(unique_part, "commit-a1b2c");
assert_eq!(remainder, "3");
```

#### Top-Level Functions

```rust
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
    K: AsRef<str>;

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
    K: AsRef<str>;
```

```rust
use id_kit::{split_id_by_unique, truncate_id_by_unique};

let target = "commit-a1b2c3";
let candidates = vec!["commit-a1b2d4", "commit-a1b999", "commit-f00000"];

let prefix = truncate_id_by_unique(target, 4, &candidates, |s| *s);
assert_eq!(prefix, "commit-a1b2c");

let (unique_part, remainder) = split_id_by_unique(target, 4, &candidates, |s| *s);
assert_eq!(unique_part, "commit-a1b2c");
assert_eq!(remainder, "3");
```

#### Self-Equality Exclusion

Candidates with identifiers identical to `target.as_ref()` are excluded from collision checks. A collection containing the target itself will not cause self-collision:

```rust
use id_kit::prelude::*;

let target = "task-42";
let candidates = vec!["task-42", "task-40", "task-50"];

// "task-42" in candidates is skipped; differentiates from "task-40" at byte 7
let prefix = target.truncate_id_by_unique(2, &candidates, |s| *s);
assert_eq!(prefix, "task-42");
```

#### Lifetime Independence

The returned values are owned `String` and `(String, String)`, completely independent of the candidate collection lifetime. The candidate collection may be dropped or modified after the operation:

```rust
use id_kit::truncate_id_by_unique;

let target = String::from("service-auth-node-1");
let prefix: String;

{
    let temporary_candidates = vec![
        String::from("service-auth-node-2"),
        String::from("service-cache-node-1"),
    ];
    prefix = truncate_id_by_unique(&target, 4, &temporary_candidates, |s| s);
    // temporary_candidates is dropped here
}

assert_eq!(prefix, "service-auth-node-1");
```

---

## Foreign DTOs and Value-Type IDs

`id-kit` works directly with external types (such as `uuid::Uuid`, `ulid::Ulid`, or RPC DTOs) without orphan-rule constraints or wrapper boilerplate.

Because `key_fn: Fn(&'a T) -> K` only requires `K: AsRef<str>`, closures can return temporary owned `String` values (`|n| n.id.to_string()`).

Additionally, because `target: impl AsRef<str>`, methods can be chained inline directly on temporary string instances without causing `E0716` ("temporary value dropped while borrowed"):

```rust
use id_kit::prelude::*;

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
        id: Uuid([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0, 0, 0, 0, 0, 0, 0, 1]),
        name: "gateway-1".to_string(),
    },
    NodeDto {
        id: Uuid([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xee, 0, 0, 0, 0, 0, 0, 0, 2]),
        name: "gateway-2".to_string(),
    },
    NodeDto {
        id: Uuid([0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0, 0, 0, 0, 0, 0, 0, 3]),
        name: "worker-1".to_string(),
    },
];

// 1. Prefix lookup with closure generating temporary String
let found = nodes.find_by_id_prefix("9988", |n| n.id.to_string())?;
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
# Ok::<(), ResolveError>(())
```

---

## UTF-8 Multi-Byte & Boundary Safety

All length parameters (`min_len`, `required_len`) represent byte counts. To prevent string slicing panics on multi-byte UTF-8 boundaries, `id-kit` implements boundary protections:

1. **Character Boundary Ceiling (`ceil_char_boundary`)**:
   When `min_len` falls within a multi-byte sequence, the prefix length snaps upward to the next valid UTF-8 character boundary.

2. **Full-Codepoint Divergence**:
   When two identifiers diverge at a multi-byte character, the entire codepoint is included in the disambiguation length.

```rust
use id_kit::prelude::*;

let target = "사용자-001";
let candidates = vec!["사용자-002", "관리자-001"];

// LCP is "사용자-00" (12 bytes). Next character '1' (1 byte) requires 13 bytes total.
let unique = target.truncate_id_by_unique(1, &candidates, |s| *s);
assert_eq!(unique, "사용자-001");

// Boundary snapping:
// min_len = 2 falls mid-character into 3-byte '사' [0..3].
// Snapped ceiling advances to byte 3 ('사').
let single_candidate = vec!["관리자-001"];
assert_eq!(target.truncate_id_by_unique(2, &single_candidate, |s| *s), "사");
```

Multi-byte safety applies across Korean (Hangul), CJK ideographs, emoji codepoints, and Latin accented characters.

---

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
