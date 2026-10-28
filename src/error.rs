use std::fmt;

/// Error returned when resolving an identifier prefix fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// No identifier matches the requested prefix.
    NotFound {
        /// The prefix that had no matching identifiers.
        prefix: String,
    },
    /// Multiple identifiers match the requested prefix.
    Ambiguous {
        /// The ambiguous prefix provided.
        prefix: String,
        /// The list of candidates matching the prefix.
        candidates: Vec<String>,
        /// The minimum prefix length needed to disambiguate among candidates.
        required_len: usize,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { prefix } => {
                write!(f, "no identifier matching prefix '{prefix}'")
            }
            Self::Ambiguous {
                prefix,
                candidates,
                required_len,
            } => {
                write!(
                    f,
                    "ambiguous prefix '{prefix}': matches {} candidates (requires {} characters to disambiguate): {:?}",
                    candidates.len(),
                    required_len,
                    candidates
                )
            }
        }
    }
}

impl std::error::Error for ResolveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_display() {
        let err = ResolveError::NotFound {
            prefix: "abc".to_string(),
        };
        assert_eq!(err.to_string(), "no identifier matching prefix 'abc'");
    }

    #[test]
    fn test_ambiguous_display() {
        let err = ResolveError::Ambiguous {
            prefix: "ab".to_string(),
            candidates: vec!["abc1".to_string(), "abc2".to_string()],
            required_len: 4,
        };
        assert_eq!(
            err.to_string(),
            "ambiguous prefix 'ab': matches 2 candidates (requires 4 characters to disambiguate): [\"abc1\", \"abc2\"]"
        );
    }

    #[test]
    fn test_error_trait() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>(_: &E) {}

        let err_nf = ResolveError::NotFound {
            prefix: "abc".to_string(),
        };
        let err_amb = ResolveError::Ambiguous {
            prefix: "ab".to_string(),
            candidates: vec!["abc1".to_string()],
            required_len: 4,
        };

        assert_error(&err_nf);
        assert_error(&err_amb);
        assert!(std::error::Error::source(&err_nf).is_none());
        assert!(std::error::Error::source(&err_amb).is_none());
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = ResolveError::NotFound {
            prefix: "abc".to_string(),
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);

        let err3 = ResolveError::NotFound {
            prefix: "xyz".to_string(),
        };
        assert_ne!(err1, err3);

        let err_amb1 = ResolveError::Ambiguous {
            prefix: "ab".to_string(),
            candidates: vec!["abc1".to_string(), "abc2".to_string()],
            required_len: 4,
        };
        let err_amb2 = err_amb1.clone();
        assert_eq!(err_amb1, err_amb2);
        assert_ne!(err1, err_amb1);

        let err_amb3 = ResolveError::Ambiguous {
            prefix: "ab".to_string(),
            candidates: vec!["abc1".to_string()],
            required_len: 4,
        };
        assert_ne!(err_amb1, err_amb3);
    }

    #[test]
    fn test_debug_format() {
        let err_nf = ResolveError::NotFound {
            prefix: "abc".to_string(),
        };
        let debug_nf = format!("{err_nf:?}");
        assert!(debug_nf.contains("NotFound"));
        assert!(debug_nf.contains("abc"));

        let err_amb = ResolveError::Ambiguous {
            prefix: "ab".to_string(),
            candidates: vec!["abc1".to_string()],
            required_len: 4,
        };
        let debug_amb = format!("{err_amb:?}");
        assert!(debug_amb.contains("Ambiguous"));
        assert!(debug_amb.contains("abc1"));
    }
}
