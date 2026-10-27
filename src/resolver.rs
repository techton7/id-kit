use crate::error::{AmbiguousCandidate, PrefixError};

/// Default minimum prefix length required for resolution (Git-style default of 8 characters).
pub const DEFAULT_MIN_PREFIX_LEN: usize = 8;

/// Computes the case-insensitive common prefix length of two byte slices.
#[inline]
pub fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes()
        .zip(b.bytes())
        .take_while(|(x, y)| x.eq_ignore_ascii_case(y))
        .count()
}

/// Resolves a prefix against a collection of ID strings using the default minimum length of 8 characters.
///
/// Matching is ASCII case-insensitive.
pub fn resolve_prefix<I, S>(prefix: &str, ids: I) -> Result<S, PrefixError<S>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str> + Clone,
{
    resolve_prefix_with(prefix, ids, DEFAULT_MIN_PREFIX_LEN)
}

/// Resolves a prefix against a collection of ID strings with a specified minimum prefix length.
///
/// Matching is ASCII case-insensitive.
pub fn resolve_prefix_with<I, S>(
    prefix: &str,
    ids: I,
    min_len: usize,
) -> Result<S, PrefixError<S>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str> + Clone,
{
    resolve_prefix_by(prefix, ids, min_len, |item| item.as_ref())
}

/// Resolves a prefix against domain items `T` using an ID selector closure `id_of`.
///
/// Returns:
/// - `Ok(item)` if exactly one candidate matches.
/// - `Err(PrefixError::TooShort)` if `prefix.trim().len() < min_len`.
/// - `Err(PrefixError::NotFound)` if 0 candidates match.
/// - `Err(PrefixError::Ambiguous)` if 2 or more candidates match. The error includes the exact
///   `required_len` (computed via pairwise Longest Common Prefix) and candidates with their
///   disambiguated prefix slices.
pub fn resolve_prefix_by<I, T, F>(
    prefix: &str,
    items: I,
    min_len: usize,
    id_of: F,
) -> Result<T, PrefixError<T>>
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> &str,
    T: Clone,
{
    let clean_prefix = prefix.trim();
    if clean_prefix.len() < min_len {
        return Err(PrefixError::TooShort {
            prefix: clean_prefix.to_string(),
            actual_len: clean_prefix.len(),
            min_len,
        });
    }

    let search_upper = clean_prefix.to_ascii_uppercase();

    // Collect all matches
    let mut matches: Vec<(T, String)> = items
        .into_iter()
        .filter_map(|item| {
            let id = id_of(&item);
            if id.to_ascii_uppercase().starts_with(&search_upper) {
                let id_str = id.to_string();
                Some((item, id_str))
            } else {
                None
            }
        })
        .collect();

    match matches.len() {
        0 => Err(PrefixError::NotFound {
            prefix: clean_prefix.to_string(),
        }),
        1 => {
            let (item, _) = matches.pop().expect("guaranteed one match");
            Ok(item)
        }
        _ => {
            // Sort matches deterministically by uppercase ID
            matches.sort_by_key(|a| a.1.to_ascii_uppercase());

            // Deduplicate if identical IDs were passed
            matches.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));

            if matches.len() == 1 {
                let (item, _) = matches.pop().expect("guaranteed one match");
                return Ok(item);
            }

            // Calculate pairwise LCP between adjacent sorted matches
            let max_pair_lcp = matches
                .windows(2)
                .map(|w| common_prefix_len(&w[0].1, &w[1].1))
                .max()
                .unwrap_or(search_upper.len());

            let max_id_len = matches
                .iter()
                .map(|(_, id)| id.len())
                .max()
                .unwrap_or(clean_prefix.len());

            let required_len = (max_pair_lcp + 1).max(min_len).min(max_id_len);

            let candidates: Vec<AmbiguousCandidate<T>> = matches
                .into_iter()
                .map(|(item, id)| {
                    let slice_len = required_len.min(id.len());
                    let disambiguated_prefix = id[..slice_len].to_string();
                    AmbiguousCandidate {
                        item,
                        id,
                        disambiguated_prefix,
                    }
                })
                .collect();

            Err(PrefixError::Ambiguous {
                prefix: clean_prefix.to_string(),
                required_len,
                candidates,
            })
        }
    }
}
