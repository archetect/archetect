use crate::case::*;
/// Converts a `&str` to `Sentence case` `String`
///
/// ```
/// use archetect_inflections::case::to_sentence_case;
///
/// assert_eq!(to_sentence_case("Foo bar"), "Foo bar");
/// assert_eq!(to_sentence_case("FooBar"), "Foo bar");
/// assert_eq!(to_sentence_case("fooBar"), "Foo bar");
/// assert_eq!(to_sentence_case("FOO_BAR"), "Foo bar");
/// assert_eq!(to_sentence_case("foo_bar"), "Foo bar");
/// assert_eq!(to_sentence_case("foo-bar"), "Foo bar");
/// ```
pub fn to_sentence_case(non_sentence_case_string: &str) -> String {
    let options = CamelOptions {
        new_word: true,
        last_char: ' ',
        first_word: true,
        injectable_char: ' ',
        has_seperator: true,
        inverted: true,
        concat_num: false,
    };
    to_case_camel_like(non_sentence_case_string, options)
}
/// Determines of a `&str` is `Sentence case`
///
/// ```
/// use archetect_inflections::case::is_sentence_case;
///
/// assert!(is_sentence_case("Foo"));
/// assert!(is_sentence_case("Foo bar string that is really really long"));
///
/// assert!(!is_sentence_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_sentence_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_sentence_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_sentence_case("Foo Bar Is A Really Really Long String"));
/// assert!(!is_sentence_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_sentence_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_sentence_case("foo"));
/// ```
pub fn is_sentence_case(test_string: &str) -> bool {
    test_string == to_sentence_case(test_string)
}

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_sentence(b: &mut Bencher) {
        b.iter(|| super::to_sentence_case("Foo BAR"));
    }

    #[bench]
    fn bench_is_sentence(b: &mut Bencher) {
        b.iter(|| super::is_sentence_case("Foo bar"));
    }

    #[bench]
    fn bench_sentence_from_snake(b: &mut Bencher) {
        b.iter(|| super::to_sentence_case("foo_bar"));
    }
}

#[cfg(test)]
mod tests {
    use super::is_sentence_case;
    use super::to_sentence_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "Foo bar".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "Robert c martin".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "Random text with bad chars".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "Trailing bad chars".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "Leading bad chars".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "Wrapped in bad chars".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "Has a sign".to_owned();
        assert_eq!(to_sentence_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_sentence_case(&convertable_string), false)
    }
}
