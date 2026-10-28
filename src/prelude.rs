//! The `id-kit` prelude.
//!
//! Re-exports all core traits, macros, error types, and convenience functions:
//!
//! - [`Identifiable`]: Trait for types identifiable by a string ID.
//! - [`impl_identifiable!`]: Macro to implement [`Identifiable`] on domain structs.
//! - [`IdSliceExt`]: Extension trait on slices providing [`.find_by_id_prefix()`](IdSliceExt::find_by_id_prefix).
//! - [`IdUniqueExt`]: Extension trait providing [`.truncate_id_by_unique()`](IdUniqueExt::truncate_id_by_unique) and [`.split_id_by_unique()`](IdUniqueExt::split_id_by_unique).
//! - [`find_by_id_prefix`]: Top-level function for resolving items by ID prefix.
//! - [`truncate_id_by_unique`]: Top-level function for truncating to a unique prefix.
//! - [`split_id_by_unique`]: Top-level function for splitting into unique prefix and remainder.
//! - [`ResolveError`]: Error type returned when prefix resolution fails.
//!
//! # Examples
//!
//! ```rust
//! use id_kit::prelude::*;
//!
//! #[derive(Debug, PartialEq, Eq)]
//! struct Resource {
//!     id: String,
//! }
//! impl_identifiable!(Resource, id);
//!
//! let resources = vec![
//!     Resource { id: "res-apple".to_string() },
//!     Resource { id: "res-apricot".to_string() },
//! ];
//!
//! let found = resources.find_by_id_prefix("res-app").unwrap();
//! assert_eq!(found.id(), "res-apple");
//! ```

pub use crate::error::ResolveError;
pub use crate::identifiable::{impl_identifiable, Identifiable};
pub use crate::lookup::{find_by_id_prefix, IdSliceExt};
pub use crate::unique::{split_id_by_unique, truncate_id_by_unique, IdUniqueExt};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct DomainItem {
        id: String,
        name: String,
    }

    // Verify impl_identifiable! macro import
    impl_identifiable!(DomainItem, id);

    #[test]
    fn test_prelude_imports() {
        let items = vec![
            DomainItem {
                id: "item-alpha-1".to_string(),
                name: "Alpha 1".to_string(),
            },
            DomainItem {
                id: "item-alpha-2".to_string(),
                name: "Alpha 2".to_string(),
            },
            DomainItem {
                id: "item-beta-1".to_string(),
                name: "Beta 1".to_string(),
            },
        ];

        // 1. Verify Identifiable trait and method
        assert_eq!(items[0].id(), "item-alpha-1");
        assert_eq!(Identifiable::id(&items[1]), "item-alpha-2");

        // 2. Verify IdSliceExt extension trait method
        let slice_res = items.find_by_id_prefix("item-beta");
        assert_eq!(slice_res, Ok(&items[2]));

        // 3. Verify find_by_id_prefix top-level function
        let fn_res = find_by_id_prefix(&items, "item-beta");
        assert_eq!(fn_res, Ok(&items[2]));

        // 4. Verify IdUniqueExt extension trait methods
        let truncated = items[0].truncate_id_by_unique(5, &items);
        assert_eq!(truncated, "item-alpha-1");

        let (unique_prefix, remainder) = items[0].split_id_by_unique(5, &items);
        assert_eq!(unique_prefix, "item-alpha-1");
        assert_eq!(remainder, "");

        let (beta_prefix, beta_rem) = items[2].split_id_by_unique(5, &items);
        assert_eq!(beta_prefix, "item-b");
        assert_eq!(beta_rem, "eta-1");

        // 5. Verify truncate_id_by_unique top-level function
        let fn_truncated = truncate_id_by_unique(&items[2], 5, &items);
        assert_eq!(fn_truncated, "item-b");

        // 6. Verify split_id_by_unique top-level function
        let (fn_prefix, fn_rem) = split_id_by_unique(&items[2], 5, &items);
        assert_eq!(fn_prefix, "item-b");
        assert_eq!(fn_rem, "eta-1");

        // 7. Verify ResolveError::NotFound
        let err_not_found = items.find_by_id_prefix("item-gamma").unwrap_err();
        match err_not_found {
            ResolveError::NotFound { prefix } => assert_eq!(prefix, "item-gamma"),
            ResolveError::Ambiguous { .. } => panic!("expected NotFound, got Ambiguous"),
        }

        // 8. Verify ResolveError::Ambiguous
        let err_ambiguous = items.find_by_id_prefix("item-alpha").unwrap_err();
        match err_ambiguous {
            ResolveError::Ambiguous {
                prefix,
                candidates,
                required_len,
            } => {
                assert_eq!(prefix, "item-alpha");
                assert_eq!(candidates.len(), 2);
                assert_eq!(required_len, 12); // "item-alpha-1" vs "item-alpha-2"
            }
            ResolveError::NotFound { .. } => panic!("expected Ambiguous, got NotFound"),
        }
    }
}
