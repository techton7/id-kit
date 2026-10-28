//! Identifiable trait and macro for string-based unique identification.
//!
//! Provides the core [`Identifiable`] trait, standard implementations for
//! string and pointer types, and the [`impl_identifiable!`] macro for
//! implementing [`Identifiable`] on domain structs.

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

/// A trait for types that can be identified by a string slice.
///
/// Any type implementing `Identifiable` provides an [`id(&self)`](Identifiable::id)
/// method returning a string slice (`&str`).
pub trait Identifiable {
    /// Returns the string identifier of the entity.
    fn id(&self) -> &str;
}

impl Identifiable for str {
    #[inline]
    fn id(&self) -> &str {
        self
    }
}

impl Identifiable for String {
    #[inline]
    fn id(&self) -> &str {
        self.as_str()
    }
}

impl<'a> Identifiable for Cow<'a, str> {
    #[inline]
    fn id(&self) -> &str {
        self.as_ref()
    }
}

impl<T: Identifiable + ?Sized> Identifiable for &T {
    #[inline]
    fn id(&self) -> &str {
        (**self).id()
    }
}

impl<T: Identifiable + ?Sized> Identifiable for Box<T> {
    #[inline]
    fn id(&self) -> &str {
        (**self).id()
    }
}

impl<T: Identifiable + ?Sized> Identifiable for Rc<T> {
    #[inline]
    fn id(&self) -> &str {
        (**self).id()
    }
}

impl<T: Identifiable + ?Sized> Identifiable for Arc<T> {
    #[inline]
    fn id(&self) -> &str {
        (**self).id()
    }
}

/// Implements [`Identifiable`] for a struct by delegating to one of its fields.
///
/// The field must itself implement [`Identifiable`] (for example `String`,
/// `&str`, `Box<str>`, `Rc<str>`, `Arc<str>`, `Cow<'_, str>`, or another struct implementing [`Identifiable`]).
///
/// # Examples
///
/// ```rust
/// use id_kit::{Identifiable, impl_identifiable};
///
/// struct Task {
///     id: String,
///     title: String,
/// }
///
/// impl_identifiable!(Task, id);
///
/// let task = Task {
///     id: "task-001".to_string(),
///     title: "Implement trait".to_string(),
/// };
/// assert_eq!(task.id(), "task-001");
/// ```
#[macro_export]
macro_rules! impl_identifiable {
    ($ty:ident < '_ >, $field:tt) => {
        impl $crate::identifiable::Identifiable for $ty<'_> {
            #[inline]
            fn id(&self) -> &str {
                $crate::identifiable::Identifiable::id(&self.$field)
            }
        }
    };
    ($ty:ident < $($life:lifetime),+ >, $field:tt) => {
        impl<$($life),+> $crate::identifiable::Identifiable for $ty<$($life),+> {
            #[inline]
            fn id(&self) -> &str {
                $crate::identifiable::Identifiable::id(&self.$field)
            }
        }
    };
    ($ty:ident < $($gen:ident),+ >, $field:tt) => {
        impl<$($gen),+> $crate::identifiable::Identifiable for $ty<$($gen),+> {
            #[inline]
            fn id(&self) -> &str {
                $crate::identifiable::Identifiable::id(&self.$field)
            }
        }
    };
    ($ty:ident, $field:tt) => {
        impl $crate::identifiable::Identifiable for $ty {
            #[inline]
            fn id(&self) -> &str {
                $crate::identifiable::Identifiable::id(&self.$field)
            }
        }
    };
}

pub use crate::impl_identifiable;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifiable_str_and_ref_str() {
        let raw_str: &str = "user-123";
        assert_eq!(raw_str.id(), "user-123");
        assert_eq!(Identifiable::id(raw_str), "user-123");
        assert_eq!(Identifiable::id(&raw_str), "user-123");

        let s: &str = "direct_str";
        assert_eq!(s.id(), "direct_str");
        assert_eq!(Identifiable::id(s), "direct_str");
    }

    #[test]
    fn test_identifiable_string() {
        let s = String::from("org-456");
        assert_eq!(s.id(), "org-456");
        assert_eq!(Identifiable::id(&s), "org-456");
    }

    #[test]
    fn test_identifiable_cow() {
        let borrowed: Cow<'_, str> = Cow::Borrowed("item-borrowed");
        assert_eq!(borrowed.id(), "item-borrowed");

        let owned: Cow<'_, str> = Cow::Owned(String::from("item-owned"));
        assert_eq!(owned.id(), "item-owned");
    }

    #[test]
    fn test_identifiable_smart_pointers() {
        let b_str: Box<str> = Box::from("box-str-id");
        assert_eq!(b_str.id(), "box-str-id");

        let b_string: Box<String> = Box::new(String::from("box-string-id"));
        assert_eq!(b_string.id(), "box-string-id");

        let rc_str: Rc<str> = Rc::from("rc-str-id");
        assert_eq!(rc_str.id(), "rc-str-id");

        let rc_string: Rc<String> = Rc::new(String::from("rc-string-id"));
        assert_eq!(rc_string.id(), "rc-string-id");

        let arc_str: Arc<str> = Arc::from("arc-str-id");
        assert_eq!(arc_str.id(), "arc-str-id");

        let arc_string: Arc<String> = Arc::new(String::from("arc-string-id"));
        assert_eq!(arc_string.id(), "arc-string-id");
    }

    #[test]
    #[allow(clippy::borrowed_box)]
    fn test_identifiable_reference_blankets() {
        let b: Box<str> = Box::from("ref-box");
        let ref_b = &b;
        assert_eq!(ref_b.id(), "ref-box");

        let r: Rc<str> = Rc::from("ref-rc");
        let ref_r = &r;
        assert_eq!(ref_r.id(), "ref-rc");

        let a: Arc<str> = Arc::from("ref-arc");
        let ref_a = &a;
        assert_eq!(ref_a.id(), "ref-arc");

        let s = "nested-ref";
        let ref1 = &s;
        let ref2 = &ref1;
        let ref3 = &ref2;
        assert_eq!(ref2.id(), "nested-ref");
        assert_eq!(ref3.id(), "nested-ref");

        let b_str: Box<str> = Box::from("ref-box-str");
        let ref_b_str: &Box<str> = &b_str;
        assert_eq!(ref_b_str.id(), "ref-box-str");
    }

    struct DomainWithString {
        id: String,
        _name: String,
    }
    impl_identifiable!(DomainWithString, id);

    struct DomainWithRef<'a> {
        id: &'a str,
        _count: usize,
    }
    impl_identifiable!(DomainWithRef<'a>, id);

    struct DomainWithAnonRef<'a> {
        id: &'a str,
    }
    impl_identifiable!(DomainWithAnonRef<'_>, id);

    struct DomainWithBox {
        id: Box<str>,
    }
    impl_identifiable!(DomainWithBox, id);

    struct DomainWithCow<'a> {
        id: Cow<'a, str>,
    }
    impl_identifiable!(DomainWithCow<'a>, id);

    struct DomainWithRc {
        id: Rc<str>,
    }
    impl_identifiable!(DomainWithRc, id);

    struct DomainWithArc {
        id: Arc<str>,
    }
    impl_identifiable!(DomainWithArc, id);

    struct NestedDomain {
        inner: DomainWithString,
    }
    impl_identifiable!(NestedDomain, inner);

    struct DeepNestedDomain {
        wrapper: NestedDomain,
    }
    impl_identifiable!(DeepNestedDomain, wrapper);

    struct TupleDomain(String, usize);
    impl_identifiable!(TupleDomain, 0);

    struct GenericDomain<T> {
        id: String,
        _extra: T,
    }
    impl_identifiable!(GenericDomain<T>, id);

    #[test]
    fn test_macro_domain_with_string() {
        let d = DomainWithString {
            id: "domain-str-1".to_string(),
            _name: "test".to_string(),
        };
        assert_eq!(d.id(), "domain-str-1");
    }

    #[test]
    fn test_macro_domain_with_ref() {
        let d = DomainWithRef {
            id: "domain-ref-1",
            _count: 42,
        };
        assert_eq!(d.id(), "domain-ref-1");

        let anon = DomainWithAnonRef {
            id: "domain-anon-1",
        };
        assert_eq!(anon.id(), "domain-anon-1");
    }

    #[test]
    fn test_macro_domain_with_box() {
        let d = DomainWithBox {
            id: Box::from("domain-box-1"),
        };
        assert_eq!(d.id(), "domain-box-1");
    }

    #[test]
    fn test_macro_domain_with_cow() {
        let d1 = DomainWithCow {
            id: Cow::Borrowed("domain-cow-borrowed"),
        };
        assert_eq!(d1.id(), "domain-cow-borrowed");

        let d2 = DomainWithCow {
            id: Cow::Owned("domain-cow-owned".to_string()),
        };
        assert_eq!(d2.id(), "domain-cow-owned");
    }

    #[test]
    fn test_macro_domain_with_rc_and_arc() {
        let d_rc = DomainWithRc {
            id: Rc::from("domain-rc-1"),
        };
        assert_eq!(d_rc.id(), "domain-rc-1");

        let d_arc = DomainWithArc {
            id: Arc::from("domain-arc-1"),
        };
        assert_eq!(d_arc.id(), "domain-arc-1");
    }

    #[test]
    fn test_macro_nested_domain() {
        let inner = DomainWithString {
            id: "nested-target-id".to_string(),
            _name: "inner".to_string(),
        };
        let outer = NestedDomain { inner };
        assert_eq!(outer.id(), "nested-target-id");

        let deep = DeepNestedDomain { wrapper: outer };
        assert_eq!(deep.id(), "nested-target-id");
    }

    #[test]
    fn test_macro_tuple_domain() {
        let t = TupleDomain("tuple-id-100".to_string(), 99);
        assert_eq!(t.id(), "tuple-id-100");
        assert_eq!(t.1, 99);
    }

    #[test]
    fn test_macro_generic_domain() {
        let g = GenericDomain {
            id: "generic-id-200".to_string(),
            _extra: vec![1, 2, 3],
        };
        assert_eq!(g.id(), "generic-id-200");
    }

    #[test]
    fn test_dynamic_dispatch() {
        let s: &str = "dyn-str";
        let string = String::from("dyn-string");
        let b_str: Box<str> = Box::from("dyn-box");
        let rc_str: Rc<str> = Rc::from("dyn-rc");
        let arc_str: Arc<str> = Arc::from("dyn-arc");
        let domain = DomainWithString {
            id: "dyn-domain".to_string(),
            _name: "test".to_string(),
        };

        let items: Vec<&dyn Identifiable> = vec![&s, &string, &b_str, &rc_str, &arc_str, &domain];

        let ids: Vec<&str> = items.into_iter().map(Identifiable::id).collect();
        assert_eq!(
            ids,
            vec![
                "dyn-str",
                "dyn-string",
                "dyn-box",
                "dyn-rc",
                "dyn-arc",
                "dyn-domain"
            ]
        );
    }

    fn accepts_identifiable<T: Identifiable>(item: &T) -> &str {
        item.id()
    }

    #[test]
    fn test_generic_function_acceptance() {
        let s: &str = "gen-str";
        assert_eq!(accepts_identifiable(&s), "gen-str");

        let string = String::from("gen-string");
        assert_eq!(accepts_identifiable(&string), "gen-string");

        let b: Box<str> = Box::from("gen-box");
        assert_eq!(accepts_identifiable(&b), "gen-box");

        let domain = DomainWithString {
            id: "gen-domain".to_string(),
            _name: "test".to_string(),
        };
        assert_eq!(accepts_identifiable(&domain), "gen-domain");
    }
}
