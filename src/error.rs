use std::fmt;

/// Candidate item details returned when a prefix matches multiple candidates ambiguously.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AmbiguousCandidate<T> {
    /// The original candidate item (or an identifier payload).
    pub item: T,
    /// The full canonical ID string.
    pub id: String,
    /// The slice of the ID of length `required_len` needed to uniquely disambiguate this candidate.
    pub disambiguated_prefix: String,
}

/// Errors returned during prefix resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PrefixError<T> {
    /// The input prefix does not satisfy the minimum character length requirement.
    TooShort {
        prefix: String,
        actual_len: usize,
        min_len: usize,
    },
    /// No candidates matched the given prefix.
    NotFound {
        prefix: String,
    },
    /// The prefix matched multiple candidates. Contains the exact `required_len`
    /// (based on pairwise Longest Common Prefix) and the list of colliding candidates.
    Ambiguous {
        prefix: String,
        required_len: usize,
        candidates: Vec<AmbiguousCandidate<T>>,
    },
}

impl<T> fmt::Display for PrefixError<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort {
                prefix,
                actual_len,
                min_len,
            } => {
                write!(
                    f,
                    "Prefix '{prefix}' has length {actual_len}, but minimum required length is {min_len} characters"
                )
            }
            Self::NotFound { prefix } => {
                write!(f, "No item found matching prefix '{prefix}'")
            }
            Self::Ambiguous {
                prefix,
                required_len,
                candidates,
            } => {
                writeln!(
                    f,
                    "Ambiguous prefix '{prefix}' matched {} items (use at least {required_len} characters to disambiguate):",
                    candidates.len()
                )?;
                for c in candidates {
                    writeln!(f, "  • {} (id: {})", c.item, c.id)?;
                }
                Ok(())
            }
        }
    }
}

impl<T> std::error::Error for PrefixError<T> where T: fmt::Debug + fmt::Display {}
