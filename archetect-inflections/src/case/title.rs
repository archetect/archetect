use crate::case::*;
/// Converts a `&str` to `Title Case` `String`
///
/// ```
/// use archetect_inflections::case::title::to_title_case;
///
/// assert!(to_title_case("Foo bar") == "Foo Bar");
/// assert!(to_title_case("FooBar") == "Foo Bar");
/// assert!(to_title_case("fooBar") == "Foo Bar");
/// assert!(to_title_case("FOO_BAR") == "Foo Bar");
/// assert!(to_title_case("foo_bar") == "Foo Bar");
/// assert!(to_title_case("foo-bar") == "Foo Bar");
/// assert!(to_title_case("foo-bar-a1a") == "Foo Bar A1a");
/// ```
pub fn to_title_case(non_title_case_string: &str) -> String {
    let options = CamelOptions {
        new_word: true,
        last_char: ' ',
        first_word: true,
        injectable_char: ' ',
        has_seperator: true,
        inverted: false,
        concat_num: false,
    };
    to_case_camel_like(non_title_case_string, options)
}

/// Determines if a `&str` is `Title Case`
///
/// ```
/// use archetect_inflections::case::title::is_title_case;
///
/// assert!(is_title_case("Foo Bar String That Is Really Really Long"));
/// assert!(!is_title_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_title_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_title_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_title_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_title_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_title_case("Foo bar string that is really really long"));
/// assert!(!is_title_case("foo"));
/// ```
pub fn is_title_case(test_string: &str) -> bool {
    test_string == to_title_case(test_string)
}

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_title(b: &mut Bencher) {
        b.iter(|| super::to_title_case("Foo BAR"));
    }

    #[bench]
    fn bench_is_title(b: &mut Bencher) {
        b.iter(|| super::is_title_case("Foo bar"));
    }

    #[bench]
    fn bench_title_from_snake(b: &mut Bencher) {
        b.iter(|| super::to_title_case("foo_bar"));
    }
}

#[cfg(test)]
mod tests {
    use super::is_title_case;
    use super::to_title_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "Foo Bar".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "Robert C Martin".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "Random Text With Bad Chars".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "Trailing Bad Chars".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "Leading Bad Chars".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "Wrapped In Bad Chars".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "Has A Sign".to_owned();
        assert_eq!(to_title_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_title_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_title_case(&convertable_string), false)
    }
}
