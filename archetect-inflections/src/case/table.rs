use crate::case::*;

use crate::string::pluralize::to_plural;

/// Converts a `&str` to `table-case` `String`
///
/// ```
/// use archetect_inflections::case::table::to_table_case;
///
/// assert_eq!(to_table_case("foo-bar"), "foo_bars");
/// assert_eq!(to_table_case("FOO_BAR"), "foo_bars");
/// assert_eq!(to_table_case("foo_bar"), "foo_bars");
/// assert_eq!(to_table_case("Foo Bar"), "foo_bars");
/// assert_eq!(to_table_case("Foo bar"), "foo_bars");
/// assert_eq!(to_table_case("FooBar"), "foo_bars");
/// assert_eq!(to_table_case("fooBar"), "foo_bars");
/// ```
pub fn to_table_case(non_table_case_string: &str) -> String {
    let snaked: String = to_case_snake_like(non_table_case_string, "_", "lower");
    let split: (&str, &str) = snaked.split_at(snaked.rfind('_').unwrap_or(0));
    format!("{}{}", split.0, to_plural(split.1))
}

/// Determines if a `&str` is `table-case`
///
/// ```
/// use archetect_inflections::case::table::is_table_case;
/// assert!(is_table_case("foo_bar_strings"));
/// assert!(!is_table_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_table_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_table_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_table_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_table_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_table_case("Foo bar string that is really really long"));
/// assert!(!is_table_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_table_case(test_string: &str) -> bool {
    to_table_case(test_string) == test_string
}

#[cfg(all(feature = "unstable", test))]

mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_table_case(b: &mut Bencher) {
        b.iter(|| super::to_table_case("Foo bar"));
    }

    #[bench]
    fn bench_is_table_case(b: &mut Bencher) {
        b.iter(|| super::is_table_case("Foo bar"));
    }
}

#[cfg(test)]

mod tests {
    use super::is_table_case;
    use super::to_table_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn from_table_case() {
        let convertable_string: String = "foo_bars".to_owned();
        let expected: String = "foo_bars".to_owned();
        assert_eq!(to_table_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_table_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_table_case() {
        let convertable_string: String = "foo_bars".to_owned();
        assert_eq!(is_table_case(&convertable_string), true)
    }
}
