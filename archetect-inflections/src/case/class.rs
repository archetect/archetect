use crate::case::*;
use crate::string::singularize::to_singular;

/// Converts a `&str` to `ClassCase` `String`
///
/// ```
/// use archetect_inflections::case::to_class_case;
///
/// assert_eq!(to_class_case("FooBar"), "FooBar");
/// assert_eq!(to_class_case("FooBars"), "FooBar");
/// assert_eq!(to_class_case("Foo Bar"), "FooBar");
/// assert_eq!(to_class_case("foo-bar"), "FooBar");
/// assert_eq!(to_class_case("fooBar"), "FooBar");
/// assert_eq!(to_class_case("FOO_BAR"), "FooBar");
/// assert_eq!(to_class_case("foo_bars"), "FooBar");
/// assert_eq!(to_class_case("Foo bar"), "FooBar");
/// ```
pub fn to_class_case(non_class_case_string: &str) -> String {
    let options = CamelOptions {
        new_word: true,
        last_char: ' ',
        first_word: false,
        injectable_char: ' ',
        has_seperator: false,
        inverted: false,
        concat_num: true,
    };
    let class_plural = to_case_camel_like(non_class_case_string, options);
    let split: (&str, &str) =
        class_plural.split_at(class_plural.rfind(char::is_uppercase).unwrap_or(0));
    format!("{}{}", split.0, to_singular(split.1))
}

/// Determines if a `&str` is `ClassCase` `bool`
///
/// ```
/// use archetect_inflections::case::is_class_case;
///
/// assert!(is_class_case("Foo"));
/// assert!(is_class_case("FooBarIsAReallyReallyLongString"));
///
/// assert!(!is_class_case("foo"));
/// assert!(!is_class_case("FooBarIsAReallyReallyLongStrings"));
/// assert!(!is_class_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_class_case("foo_bar_is_a_really_really_long_strings"));
/// assert!(!is_class_case("fooBarIsAReallyReallyLongString"));
/// assert!(!is_class_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_class_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_class_case("Foo bar string that is really really long"));
/// assert!(!is_class_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_class_case(test_string: &str) -> bool {
    to_class_case(test_string) == test_string
}

#[cfg(test)]
mod tests {
    use super::is_class_case;
    use super::to_class_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_screaming_class_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_table_case() {
        let convertable_string: String = "foo_bars".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "FooBar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "RobertCMartin".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "RandomTextWithBadChar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "TrailingBadChar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "LeadingBadChar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "WrappedInBadChar".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "HasASign".to_owned();
        assert_eq!(to_class_case(&convertable_string), expected)
    }

    #[test]
    fn is_correct_from_class_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_class_case(&convertable_string), true)
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert_eq!(is_class_case(&convertable_string), false)
    }

    #[test]
    fn is_correct_from_table_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert_eq!(is_class_case(&convertable_string), true)
    }
}
