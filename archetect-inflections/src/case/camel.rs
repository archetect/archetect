use crate::case::*;

/// Converts a `&str` to camelCase `String`
///
/// ```
/// use archetect_inflections::case::to_camel_case;
///
/// assert_eq!(to_camel_case("fooBar"), "fooBar");
/// assert_eq!(to_camel_case("FOO_BAR"), "fooBar");
/// assert_eq!(to_camel_case("Foo Bar"), "fooBar");
/// assert_eq!(to_camel_case("foo_bar"), "fooBar");
/// assert_eq!(to_camel_case("Foo bar"), "fooBar");
/// assert_eq!(to_camel_case("foo-bar"), "fooBar");
/// assert_eq!(to_camel_case("FooBar"), "fooBar");
/// assert_eq!(to_camel_case("FooBar3"), "fooBar3");
/// assert_eq!(to_camel_case("Foo-Bar"), "fooBar");
/// ```
pub fn to_camel_case(non_camelized_string: &str) -> String {
    let options = CamelOptions {
        new_word: false,
        last_char: ' ',
        first_word: false,
        injectable_char: ' ',
        has_seperator: false,
        inverted: false,
        concat_num: true,
    };
    to_case_camel_like(non_camelized_string, options)
}

/// Determines if a `&str` is camelCase bool``
///
/// ```
/// use archetect_inflections::case::is_camel_case;
///
/// assert!(is_camel_case("foo"));
/// assert!(is_camel_case("fooBarIsAReallyReally3LongString"));
/// assert!(is_camel_case("fooBarIsAReallyReallyLongString"));
///
/// assert!(!is_camel_case("Foo"));
/// assert!(!is_camel_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_camel_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_camel_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_camel_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_camel_case("Foo bar string that is really really long"));
/// assert!(!is_camel_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_camel_case(test_string: &str) -> bool {
    to_camel_case(test_string) == test_string
}

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_camel0(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "Foo bar";
            super::to_camel_case(test_string)
        });
    }

    #[bench]
    fn bench_camel1(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "foo_bar";
            super::to_camel_case(test_string)
        });
    }

    #[bench]
    fn bench_camel2(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "fooBar";
            super::to_camel_case(test_string)
        });
    }

    #[bench]
    fn bench_is_camel(b: &mut Bencher) {
        b.iter(|| {
            let test_string: &str = "Foo bar";
            super::is_camel_case(test_string)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::is_camel_case;
    use super::to_camel_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "robertCMartin".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "randomTextWithBadChars".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "trailingBadChars".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "leadingBadChars".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "wrappedInBadChars".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "hasASign".to_owned();
        assert_eq!(to_camel_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_camel_case(&convertable_string), false)
    }
}
