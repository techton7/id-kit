use std::collections::HashMap;

use crate::resolver::{common_prefix_len, DEFAULT_MIN_PREFIX_LEN};

/// Abbreviates a collection of unique identifiers (such as UUIDs, ULIDs, or hashes)
/// to their shortest unambiguous prefix.
///
/// Pre-calculates the disambiguated length in O(N log N) time based on the pairwise
/// Longest Common Prefix (LCP) between sorted candidates.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UidAbbreviator {
    min_lengths: HashMap<String, usize>,
    min_len: usize,
}

impl UidAbbreviator {
    /// Builds a `UidAbbreviator` from an iterator of ID strings with the default minimum length of 8 characters.
    pub fn from_ids<'a, I>(ids: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        Self::with_min_len(ids, DEFAULT_MIN_PREFIX_LEN)
    }

    /// Builds a `UidAbbreviator` from an iterator of ID strings with a specified minimum prefix length.
    pub fn with_min_len<'a, I>(ids: I, min_len: usize) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut sorted: Vec<&'a str> = ids.into_iter().collect();
        sorted.sort_unstable_by_key(|a| a.to_ascii_uppercase());
        sorted.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        let mut min_lengths = HashMap::with_capacity(sorted.len());
        for i in 0..sorted.len() {
            let lcp_prev = if i > 0 {
                common_prefix_len(sorted[i - 1], sorted[i])
            } else {
                0
            };
            let lcp_next = if i + 1 < sorted.len() {
                common_prefix_len(sorted[i], sorted[i + 1])
            } else {
                0
            };
            let len = (lcp_prev.max(lcp_next) + 1).max(min_len).min(sorted[i].len());
            min_lengths.insert(sorted[i].to_string(), len);
        }

        Self {
            min_lengths,
            min_len,
        }
    }

    /// Returns the shortest unambiguous prefix slice for the given ID.
    ///
    /// If the ID was not present in the original set, returns a slice clamped to `min_len`.
    #[inline]
    pub fn abbreviate<'a>(&self, id: &'a str) -> &'a str {
        let len = self
            .min_lengths
            .get(id)
            .copied()
            .unwrap_or(self.min_len)
            .min(id.len());
        &id[..len]
    }

    /// Returns the precomputed disambiguated length for the given ID.
    #[inline]
    pub fn disambiguated_len(&self, id: &str) -> usize {
        self.min_lengths.get(id).copied().unwrap_or(self.min_len)
    }

    /// Returns the configured minimum prefix length.
    #[inline]
    pub fn min_len(&self) -> usize {
        self.min_len
    }

    /// Returns the total number of IDs tracked.
    #[inline]
    pub fn len(&self) -> usize {
        self.min_lengths.len()
    }

    /// Returns true if no IDs are tracked.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min_lengths.is_empty()
    }
}
