use crate::case::*;
/// Converts a `&str` to pascalCase `String`
///
/// ```
/// use archetect_inflections::case::to_pascal_case;
///
/// assert_eq!(to_pascal_case("fooBar"), "FooBar");
/// assert_eq!(to_pascal_case("FOO_BAR"), "FooBar");
/// assert_eq!(to_pascal_case("Foo Bar"), "FooBar");
/// assert_eq!(to_pascal_case("foo_bar"), "FooBar");
/// assert_eq!(to_pascal_case("Foo bar"), "FooBar");
/// assert_eq!(to_pascal_case("foo-bar"), "FooBar");
/// assert_eq!(to_pascal_case("FooBar"), "FooBar");
/// assert_eq!(to_pascal_case("FooBar3"), "FooBar3");
/// assert_eq!(to_pascal_case("FooBar3a"), "FooBar3a");
/// ```
pub fn to_pascal_case(non_pascalized_string: &str) -> String {
    let options = CamelOptions {
        new_word: true,
        last_char: ' ',
        first_word: false,
        injectable_char: ' ',
        has_seperator: false,
        inverted: false,
        concat_num: false,
    };
    to_case_camel_like(non_pascalized_string, options)
}

/// Determines if a `&str` is pascalCase bool``
///
/// ```
/// use archetect_inflections::case::is_pascal_case;
///
/// assert!(is_pascal_case("Foo"));
/// assert!(is_pascal_case("FooBarIsAReallyReallyLongString"));
/// assert!(is_pascal_case("FooBarIsAReallyReally3longString"));
/// assert!(is_pascal_case("FooBarIsAReallyReallyLongString"));
///
/// assert!(!is_pascal_case("foo"));
/// assert!(!is_pascal_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_pascal_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_pascal_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_pascal_case("Foo bar string that is really really long"));
/// assert!(!is_pascal_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_pascal_case(test_string: &str) -> bool {
    to_pascal_case(test_string) == test_string
}

#[cfg(all(feature = "unstable", test))]
mod benchmarks {
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_pascal0(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "Foo bar";
            super::to_pascal_case(test_string)
        });
    }

    #[bench]
    fn bench_pascal1(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "foo_bar";
            super::to_pascal_case(test_string)
        });
    }

    #[bench]
    fn bench_pascal2(b: &mut Bencher) {
        b.iter(|| {
            let test_string = "fooBar";
            super::to_pascal_case(test_string)
        });
    }

    #[bench]
    fn bench_is_pascal(b: &mut Bencher) {
        b.iter(|| {
            let test_string: &str = "Foo bar";
            super::is_pascal_case(test_string)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::is_pascal_case;
    use super::to_pascal_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "RobertCMartin".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "RandomTextWithBadChars".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "TrailingBadChars".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "LeadingBadChars".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "WrappedInBadChars".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "HasASign".to_owned();
        assert_eq!(to_pascal_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_pascal_case(&convertable_string), false)
    }
}
