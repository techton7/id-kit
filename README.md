# id-kit

[![Crates.io](https://img.shields.io/crates/v/id-kit.svg)](https://crates.io/crates/id-kit)
[![Documentation](https://docs.rs/id-kit/badge.svg)](https://docs.rs/id-kit)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Zero-dependency, `#![forbid(unsafe_code)]` Rust toolkit specializing strictly in:

1. Prefix-based ID lookup (`find_by_id_prefix` / `candidates.find_by_id_prefix(...)`)
2. Minimal unique prefix truncation and splitting (`truncate_id_by_unique`, `split_id_by_unique` / `target.truncate_id_by_unique(...)`, `target.split_id_by_unique(...)`)

## Key Properties

- **Zero External Dependencies**: Implemented using standard library primitives (`std` only).
- **Exact Matching**: Prefix lookups use case-sensitive exact matching (`starts_with`) without implicit transformations or trimming.
- **UTF-8 Safety**: Length calculations operate on byte offsets while enforcing valid Unicode scalar value boundaries via character boundary ceiling (`ceil_char_boundary`). Slicing panics on multi-byte codepoints are prevented.
- **Memory Safety**: Enforced with `#![forbid(unsafe_code)]`.
- **Lifetime Independence**: Truncation and splitting return string slices tied to the lifetime of the target identifier, independent of candidate collections.

---

## Installation

Add the dependency to `Cargo.toml`:

```toml
[dependencies]
id-kit = "0.1.1"
```

---

## Quickstart

Import the prelude to bring core traits, functions, and macros into scope:

```rust
use id_kit::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Task {
    id: String,
    title: String,
}

impl_identifiable!(Task, id);

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
    match tasks.find_by_id_prefix("task-d") {
        Err(ResolveError::Ambiguous { prefix, candidates, required_len }) => {
            assert_eq!(prefix, "task-d");
            assert_eq!(candidates.len(), 2);
            assert_eq!(required_len, 13); // Disambiguation requires "task-deploy-w"
        }
        _ => unreachable!(),
    }

    // Lookup with disambiguated prefix resolves uniquely
    let task = tasks.find_by_id_prefix("task-deploy-w")?;
    assert_eq!(task.title, "Deploy web service");

    // 2. Minimal unique prefix truncation (min_len: 5)
    let unique_prefix = task.truncate_id_by_unique(5, &tasks);
    assert_eq!(unique_prefix, "task-deploy-w");

    // 3. Minimal unique prefix splitting
    let (prefix, remainder) = task.split_id_by_unique(5, &tasks);
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

Available on slices `[T]` where `T: Identifiable`:

```rust
use id_kit::prelude::*;

let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

let matched = candidates.find_by_id_prefix("item-alpi")?;
assert_eq!(*matched, "item-alpine");
# Ok::<(), ResolveError>(())
```

#### Top-Level Function

```rust
use id_kit::find_by_id_prefix;

let candidates = vec!["item-alpha", "item-alpine", "item-beta"];

let matched = find_by_id_prefix(&candidates, "item-alpi")?;
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

match candidates.find_by_id_prefix("item-alp") {
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

Available on any type implementing `IdUniqueExt` (blanket implementation for all `T: Identifiable`):

- `target.truncate_id_by_unique(min_len, &candidates) -> &str`
- `target.split_id_by_unique(min_len, &candidates) -> (&str, &str)`

```rust
use id_kit::prelude::*;

let target = "commit-a1b2c3";
let candidates = vec!["commit-a1b2d4", "commit-a1b999", "commit-f00000"];

// Truncate to minimal unique prefix (with min_len = 4)
let prefix = target.truncate_id_by_unique(4, &candidates);
assert_eq!(prefix, "commit-a1b2c");

// Split into unique prefix and remaining suffix
let (unique_part, remainder) = target.split_id_by_unique(4, &candidates);
assert_eq!(unique_part, "commit-a1b2c");
assert_eq!(remainder, "3");
```

#### Top-Level Functions

```rust
use id_kit::{truncate_id_by_unique, split_id_by_unique};

let target = "commit-a1b2c3";
let candidates = vec!["commit-a1b2d4", "commit-a1b999", "commit-f00000"];

let prefix = truncate_id_by_unique(&target, 4, &candidates);
assert_eq!(prefix, "commit-a1b2c");

let (unique_part, remainder) = split_id_by_unique(&target, 4, &candidates);
assert_eq!(unique_part, "commit-a1b2c");
assert_eq!(remainder, "3");
```

#### Self-Equality Exclusion

Candidates with identifiers identical to `target.id()` are excluded from collision checks. A collection containing the target itself will not cause self-collision:

```rust
use id_kit::prelude::*;

let target = "task-42";
let candidates = vec!["task-42", "task-40", "task-50"];

// "task-42" in candidates is skipped; differentiates from "task-40" at byte 7
let prefix = target.truncate_id_by_unique(2, &candidates);
assert_eq!(prefix, "task-42");
```

#### Lifetime Guarantees

The returned string slice borrows from `&'a target`, not from `candidates`. The candidate collection may be dropped or modified while the returned prefix remains valid:

```rust
use id_kit::truncate_id_by_unique;

let target = String::from("service-auth-node-1");
let prefix: &str;

{
    let temporary_candidates = vec![
        String::from("service-auth-node-2"),
        String::from("service-cache-node-1"),
    ];
    prefix = truncate_id_by_unique(&target, 4, &temporary_candidates);
    // temporary_candidates is dropped here
}

assert_eq!(prefix, "service-auth-node-1");
```

---

## Identifiable Trait & Macro

The `Identifiable` trait defines a uniform interface for string identifier access:

```rust
pub trait Identifiable {
    fn id(&self) -> &str;
}
```

### Standard Implementations

`Identifiable` is implemented out of the box for:
- `str`
- `&str`
- `String`
- `Cow<'_, str>`
- `Box<T>` where `T: Identifiable + ?Sized`
- `Rc<T>` where `T: Identifiable + ?Sized`
- `Arc<T>` where `T: Identifiable + ?Sized`
- `&T` where `T: Identifiable + ?Sized` (blanket reference implementation)

### Deriving with `impl_identifiable!`

The `impl_identifiable!` macro implements `Identifiable` for structs by delegating to a specified field or tuple index:

```rust
use id_kit::prelude::*;

// Standard struct with owned String
struct Task {
    id: String,
    title: String,
}
impl_identifiable!(Task, id);

// Struct with explicit lifetime parameter
struct RefTask<'a> {
    id: &'a str,
}
impl_identifiable!(RefTask<'a>, id);

// Struct with anonymous lifetime
struct AnonTask<'a> {
    id: &'a str,
}
impl_identifiable!(AnonTask<'_>, id);

// Generic struct
struct GenericTask<T> {
    id: String,
    data: T,
}
impl_identifiable!(GenericTask<T>, id);

// Tuple struct
struct TupleTask(String, usize);
impl_identifiable!(TupleTask, 0);
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
let unique = target.truncate_id_by_unique(1, &candidates);
assert_eq!(unique, "사용자-001");

// Boundary snapping:
// min_len = 2 falls mid-character into 3-byte '사' [0..3].
// Snapped ceiling advances to byte 3 ('사').
let single_candidate = vec!["관리자-001"];
assert_eq!(target.truncate_id_by_unique(2, &single_candidate), "사");
```

Multi-byte safety applies across Korean (Hangul), CJK ideographs, emoji codepoints, and Latin accented characters.

---

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
