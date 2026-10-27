# uid-prefix

[![Crates.io](https://img.shields.io/crates/v/uid-prefix.svg)](https://crates.io/crates/uid-prefix)
[![Documentation](https://docs.rs/uid-prefix/badge.svg)](https://docs.rs/uid-prefix)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

**Fast, zero-dependency Git-style short UID resolution, pairwise LCP collision disambiguation, and dynamic prefix abbreviation in Rust.**

Designed for CLI applications, databases, and developer tools operating on UUIDs, ULIDs, Git commit hashes, or any string-based unique identifiers.

---

## Features

- **Git-style Short Prefix Resolution**: Resolves case-insensitive prefixes against candidate collections using a configurable minimum length (`default: 8`).
- **Pairwise LCP Collision Diagnostics**: When a prefix collision occurs, computes the exact pairwise Longest Common Prefix (LCP) between colliding candidates and reports the exact character length required to disambiguate.
- **Dynamic ID Abbreviation (`UidAbbreviator`)**: Pre-computes the shortest collision-free prefix for every ID in a collection in $O(N \log N)$ time, ideal for compact terminal display tables and logs.
- **Zero Runtime Dependencies**: Pure Rust standard library by default. Optional `serde` feature available.
- **Format Agnostic**: Works out of the box with UUIDv4/v7, ULID, Crockford Base32, Hex hashes, or custom string IDs.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
uid-prefix = "0.1"

# With optional serde support for PrefixError and AmbiguousCandidate:
# uid-prefix = { version = "0.1", features = ["serde"] }
```

---

## Usage

### 1. Simple ID Prefix Resolution

```rust
use uid_prefix::{resolve_prefix, PrefixError};

let uids = vec![
    "01M5VDRGAAAA",
    "01M5VDRGBBBB",
    "01M5VDV4CCCC",
];

// Resolves uniquely with minimum 8 characters
let matched = resolve_prefix("01M5VDV4", &uids).unwrap();
assert_eq!(*matched, "01M5VDV4CCCC");

// Prefix collision detection with exact required length
match resolve_prefix("01M5VDRG", &uids) {
    Err(PrefixError::Ambiguous { required_len, candidates, .. }) => {
        // Collided up to 8 characters ("01M5VDRG"), requires 9 to disambiguate
        assert_eq!(required_len, 9);
        assert_eq!(candidates.len(), 2);
        println!("Candidates: {:?}", candidates);
    }
    _ => unreachable!(),
}
```

### 2. Resolving Custom Domain Objects (`resolve_prefix_by`)

Avoid intermediate Lookups by resolving directly against your domain structs:

```rust
use uid_prefix::{resolve_prefix_by, PrefixError};

#[derive(Clone, Debug)]
struct Task {
    id: String,
    title: String,
}

let tasks = vec![
    Task { id: "0194a1b2-3c4d-4e5f-8a9b-0c1d2e3f4a5b".into(), title: "Parser".into() },
    Task { id: "0194a1b2-3f9a-4e5f-8a9b-0c1d2e3f4a5b".into(), title: "CLI".into() },
];

let res = resolve_prefix_by("0194a1b2-3c", &tasks, 8, |t| &t.id);
assert_eq!(res.unwrap().title, "Parser");
```

### 3. Dynamic Prefix Abbreviation (`UidAbbreviator`)

Display shortened IDs in your CLI without risk of ambiguity:

```rust
use uid_prefix::UidAbbreviator;

let uids = vec![
    "01M5VDRG2E2DVBXJST67N1JWPD",
    "01M5VDRPDYPZ471ZZ1X13CYQQF",
    "01M5VDV4NY1NSKDK3XEQ10QYB7",
];

let abbr = UidAbbreviator::from_ids(uids.iter().copied());

// Automatically computes the shortest collision-free prefix for each item:
assert_eq!(abbr.abbreviate(uids[0]), "01M5VDRG");
assert_eq!(abbr.abbreviate(uids[1]), "01M5VDRP");
assert_eq!(abbr.abbreviate(uids[2]), "01M5VDV4");
```

---

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
