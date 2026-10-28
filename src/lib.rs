//! # id-kit
//!
//! Pure zero-dependency Rust toolkit for Git-style short identifier resolution,
//! longest common prefix (LCP) collision disambiguation, and unique abbreviation.
//!
//! Built with `#![forbid(unsafe_code)]` for 100% memory-safe operation without any
//! runtime dependencies.
//!
//! ## Core Operations
//!
//! 1. **Prefix Lookup**:
//!    Resolve a unique entity from a collection of candidates given an identifier
//!    prefix using [`find_by_id_prefix`] or the [`IdSliceExt::find_by_id_prefix`]
//!    extension method.
//!    - Returns `Ok(&T)` if exactly one candidate matches.
//!    - Returns [`ResolveError::NotFound`] if no candidate matches.
//!    - Returns [`ResolveError::Ambiguous`] if two or more candidates match,
//!      providing conflicting IDs and the minimum prefix length (`required_len`)
//!      needed to disambiguate them.
//!
//! 2. **Unique Prefix Truncation & Splitting**:
//!    Compute the minimal unique prefix that distinguishes an identifier from all
//!    other candidates in a collection using [`truncate_id_by_unique`],
//!    [`split_id_by_unique`], or the [`IdUniqueExt`] extension trait methods.
//!    All operations respect UTF-8 character boundaries and enforce a configurable
//!    minimum prefix length.
//!
//! ## Idiomatic Usage
//!
//! Bring all core traits, extension methods, functions, and macros into scope:
//!
//! ```rust
//! use id_kit::prelude::*;
//!
//! // 1. Define a domain struct and implement Identifiable
//! #[derive(Debug, PartialEq, Eq)]
//! struct Commit {
//!     hash: String,
//!     message: String,
//! }
//!
//! impl_identifiable!(Commit, hash);
//!
//! let commits = vec![
//!     Commit { hash: "a1b2c3d4".to_string(), message: "Initial commit".to_string() },
//!     Commit { hash: "a1b2e5f6".to_string(), message: "Fix parser bug".to_string() },
//!     Commit { hash: "f9e8d7c6".to_string(), message: "Release v1.0".to_string() },
//! ];
//!
//! // 2. Prefix lookup via slice extension trait
//! let found = commits.find_by_id_prefix("f9e")?;
//! assert_eq!(found.message, "Release v1.0");
//!
//! // 3. Ambiguity handling: when a prefix matches multiple candidates,
//! // ResolveError::Ambiguous reports candidates and the required disambiguation length
//! match commits.find_by_id_prefix("a1b2") {
//!     Err(ResolveError::Ambiguous { prefix, candidates, required_len }) => {
//!         assert_eq!(prefix, "a1b2");
//!         assert_eq!(candidates.len(), 2);
//!         assert_eq!(required_len, 5); // "a1b2c" vs "a1b2e"
//!     }
//!     _ => unreachable!(),
//! }
//!
//! // Disambiguated query succeeds
//! let disambiguated = commits.find_by_id_prefix("a1b2c")?;
//! assert_eq!(disambiguated.message, "Initial commit");
//!
//! // 4. Unique prefix truncation via item extension trait
//! let short = commits[0].truncate_id_by_unique(3, &commits);
//! assert_eq!(short, "a1b2c");
//!
//! // 5. Unique prefix splitting into (prefix, remainder)
//! let (prefix, remainder) = commits[0].split_id_by_unique(3, &commits);
//! assert_eq!(prefix, "a1b2c");
//! assert_eq!(remainder, "3d4");
//!
//! // 6. Top-level function equivalents
//! let looked_up = find_by_id_prefix(&commits, "a1b2c")?;
//! assert_eq!(looked_up.message, "Initial commit");
//!
//! let truncated = truncate_id_by_unique(&commits[2], 4, &commits);
//! assert_eq!(truncated, "f9e8");
//!
//! let (p, r) = split_id_by_unique(&commits[2], 4, &commits);
//! assert_eq!(p, "f9e8");
//! assert_eq!(r, "d7c6");
//!
//! # Ok::<(), ResolveError>(())
//! ```

#![forbid(unsafe_code)]

pub mod error;
pub mod identifiable;
pub mod lookup;
pub mod prelude;
pub mod unique;

pub use error::ResolveError;
pub use identifiable::Identifiable;
pub use lookup::{find_by_id_prefix, IdSliceExt};
pub use unique::{split_id_by_unique, truncate_id_by_unique, IdUniqueExt};
