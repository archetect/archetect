use crate::case::{CamelOptions, to_case_camel_like};

/// Determines if a `&str` is `directory/case`
///
/// ```
/// use archetect_inflections::case::is_directory_case;
///
/// assert!(is_directory_case("foo/bar/string/that/is/really/really/long"));
/// assert!(is_directory_case("foo/bar/string/that/is/really3a/really/long"));
/// assert!(!is_directory_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_directory_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_directory_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_directory_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_directory_case("Foo bar string that is really really long"));
/// assert!(!is_directory_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_directory_case(test_string: &str) -> bool {
    test_string == to_directory_case(test_string)
}

/// Converts a `&str` to `kebab-case` `String`
///
/// ```
/// use archetect_inflections::case::to_directory_case;
///
/// assert_eq!(to_directory_case("foo-bar"), "foo/bar");
/// assert_eq!(to_directory_case("FOO_BAR"), "foo/bar");
/// assert_eq!(to_directory_case("foo_bar"), "foo/bar");
/// assert_eq!(to_directory_case("Foo Bar"),"foo/bar");
/// assert_eq!(to_directory_case("Foo bar"), "foo/bar");
/// assert_eq!(to_directory_case("FooBar"), "foo/bar");
/// assert_eq!(to_directory_case("fooBar"), "foo/bar");
/// assert_eq!(to_directory_case("fooBar3a"), "foo/bar3a");
/// ```
pub fn to_directory_case(input: &str) -> String {
    let options = CamelOptions {
        new_word: false,
        last_char: ' ',
        first_word: false,
        injectable_char: '/',
        has_seperator: true,
        inverted: true,
        concat_num: false,
    };
    to_case_camel_like(input, options)
}