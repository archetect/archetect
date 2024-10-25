use crate::case::*;
/// Determines if a `&str` is `kebab-case`
///
/// ```
/// use archetect_inflections::case::is_kebab_case;
///
/// assert!(is_kebab_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_kebab_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_kebab_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_kebab_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_kebab_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_kebab_case("Foo bar string that is really really long"));
/// assert!(!is_kebab_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_kebab_case(test_string: &str) -> bool {
    test_string == to_kebab_case(test_string)
}

/// Converts a `&str` to `kebab-case` `String`
///
/// ```
/// use archetect_inflections::case::to_kebab_case;
///
/// assert_eq!(to_kebab_case("foo-bar"), "foo-bar");
/// assert_eq!(to_kebab_case("FOO_BAR"), "foo-bar");
/// assert_eq!(to_kebab_case("foo_bar"), "foo-bar");
/// assert_eq!(to_kebab_case("Foo Bar"),"foo-bar");
/// assert_eq!(to_kebab_case("Foo bar"), "foo-bar");
/// assert_eq!(to_kebab_case("FooBar"), "foo-bar");
/// assert_eq!(to_kebab_case("fooBar"), "foo-bar");
/// assert_eq!(to_kebab_case("fooBar3"), "foo-bar3");
/// assert_eq!(to_kebab_case("p6m-dev"), "p6m-dev");
/// ```
pub fn to_kebab_case(non_kebab_case_string: &str) -> String {
    let options = CamelOptions {
        new_word: false,
        last_char: ' ',
        first_word: false,
        injectable_char: '-',
        has_seperator: true,
        inverted: true,
        concat_num: false,
    };
    to_case_camel_like(non_kebab_case_string, options)
}

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_kebab(b: &mut Bencher) {
        b.iter(|| super::to_kebab_case("Foo bar"));
    }

    #[bench]
    fn bench_is_kebab(b: &mut Bencher) {
        b.iter(|| super::is_kebab_case("Foo bar"));
    }

    #[bench]
    fn bench_kebab_from_snake(b: &mut Bencher) {
        b.iter(|| super::to_kebab_case("test_test_test"));
    }
}

#[cfg(test)]
mod tests {
    use super::is_kebab_case;
    use super::to_kebab_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "foo-bar".to_owned();
        assert_eq!(to_kebab_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_kebab_case(&convertable_string), false)
    }
}
