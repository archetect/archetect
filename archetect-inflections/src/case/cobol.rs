use crate::to_kebab_case;

/// Determines if a `&str` is `COBOL-CASE`
///
/// ```
/// use archetect_inflections::case::is_cobol_case;
///
/// assert!(is_cobol_case("FOO-BAR-STRING-THAT-IS-REALLY-REALLY-LONG"));
/// assert!(!is_cobol_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_cobol_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_cobol_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_cobol_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_cobol_case("Foo bar string that is really really long"));
/// assert!(!is_cobol_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_cobol_case(test_string: &str) -> bool {
    test_string == to_cobol_case(test_string)
}

/// Converts a `&str` to `COBOL-CASE` `String`
///
/// ```
/// use archetect_inflections::case::to_cobol_case;
///
/// assert_eq!(to_cobol_case("foo-bar"), "FOO-BAR");
/// assert_eq!(to_cobol_case("FOO_BAR"), "FOO-BAR");
/// assert_eq!(to_cobol_case("foo_bar"), "FOO-BAR");
/// assert_eq!(to_cobol_case("Foo Bar"),"FOO-BAR");
/// assert_eq!(to_cobol_case("Foo bar"), "FOO-BAR");
/// assert_eq!(to_cobol_case("FooBar"), "FOO-BAR");
/// assert_eq!(to_cobol_case("fooBar"), "FOO-BAR");
/// assert_eq!(to_cobol_case("fooBar3"), "FOO-BAR3");
/// assert_eq!(to_cobol_case("fooBar3a"), "FOO-BAR3A");
/// ```
pub fn to_cobol_case(input: &str) -> String {
    to_kebab_case(input).to_uppercase()
}