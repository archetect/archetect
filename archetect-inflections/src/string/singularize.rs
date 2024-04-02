use std::sync::OnceLock;
use regex::Regex;

use crate::string::constants::UNACCONTABLE_WORDS;

macro_rules! special_cases{
    ($s:ident, $($singular: expr => $plural:expr), *) => {
        match &$s[..] {
            $(
                $singular => {
                    return $plural.to_owned();
                },
            )*
            _ => ()
        }
    }
}

/// Converts a `&str` to singularized `String`
///
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "foo_bars";
/// let expected_string: String = "foo_bar".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "oxen";
/// let expected_string: String = "ox".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "crates";
/// let expected_string: String = "crate".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "oxen";
/// let expected_string: String = "ox".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "boxes";
/// let expected_string: String = "box".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "vengeance";
/// let expected_string: String = "vengeance".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
/// ```
/// use archetect_inflections::string::singularize::to_singular;
/// let mock_string: &str = "yoga";
/// let expected_string: String = "yoga".to_owned();
/// let asserted_string: String = to_singular(mock_string);
/// assert!(asserted_string == expected_string);
///
/// ```
///
pub fn to_singular(non_singular_string: &str) -> String {
    if UNACCONTABLE_WORDS.contains(&non_singular_string) {
        non_singular_string.to_owned()
    } else {
        special_cases![non_singular_string,
            "oxen" => "ox",
            "boxes" => "box",
            "men" => "man",
            "women" => "woman",
            "dice" => "die",
            "yeses" => "yes",
            "feet" => "foot",
            "eaves" => "eave",
            "geese" => "goose",
            "teeth" => "tooth",
            "quizzes" => "quiz"
        ];
        for &(ref rule, replace) in patterns().iter().rev() {
            if let Some(captures) = rule.captures(non_singular_string) {
                if let Some(c) = captures.get(1) {
                    let mut buf = String::new();
                    captures.expand(&format!("{}{}", c.as_str(), replace), &mut buf);
                    return buf;
                }
            }
        }

        non_singular_string.to_owned()
    }
}

fn patterns() -> &'static Vec<(Regex, &'static str)> {
    static REGEX: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEX.get_or_init(||vec![
        (r"(\w*)s$", ""),
                                     (r"(\w*(ss))$", ""),
                                     (r"(\w*(n))ews$", "ews"),
                                     (r"(\w*(o))es$", ""),
                                     (r"(\w*([ti]))a$", "um"),
                                     (
                                         r"(\w*((a)naly|(b)a|(d)iagno|(p)arenthe|(p)rogno|(s)ynop|(t)he))(sis|ses)$",
                                         "sis",
                                     ),
                                     (r"(^analy)(sis|ses)$", "sis"),
                                     (r"(\w*([^f]))ves$", "fe"),
                                     (r"(\w*(hive))s$", ""),
                                     (r"(\w*(tive))s$", ""),
                                     (r"(\w*([lr]))ves$", "f"),
                                     (r"(\w*([^aeiouy]|qu))ies$", "y"),
                                     (r"(\w*(s))eries$", "eries"),
                                     (r"(\w*(m))ovies$", "ovie"),
                                     (r"(\w*(x|ch|ss|sh))es$", ""),
                                     (r"(\w*(m|l))ice$", "ouse"),
                                     (r"(\w*(bus))(es)?$", ""),
                                     (r"(\w*(shoe))s$", ""),
                                     (r"(\w*(cris|test))(is|es)$", "is"),
                                     (r"^(a)x[ie]s$", "xis"),
                                     (r"(\w*(octop|vir))(us|i)$", "us"),
                                     (r"(\w*(alias|status))(es)?$", ""),
                                     (r"^(ox)en", ""),
                                     (r"(\w*(vert|ind))ices$", "ex"),
                                     (r"(\w*(matr))ices$", "ix"),
                                     (r"(\w*(quiz))zes$", ""),
                                     (r"(\w*(database))s$", ""),].into_iter().map(|(rule, replace)| {(Regex::new(rule).unwrap(), replace)}).collect())
}

#[test]
fn singularize_ies_suffix() {
    assert_eq!("reply", to_singular("replies"));
    assert_eq!("lady", to_singular("ladies"));
    assert_eq!("soliloquy", to_singular("soliloquies"));
}

#[test]
fn singularize_ss_suffix() {
    assert_eq!("glass", to_singular("glass"));
    assert_eq!("access", to_singular("access"));
    assert_eq!("glass", to_singular("glasses"));
    assert_eq!("witch", to_singular("witches"));
    assert_eq!("dish", to_singular("dishes"));
}

#[test]
fn singularize_string_if_a_regex_will_match() {
    assert_eq!("news", to_singular("news"));
    assert_eq!("goodnews", to_singular("goodnews"));
    assert_eq!("potato", to_singular("potatoes"));
    assert_eq!("datum", to_singular("data"));
    assert_eq!("analysis", to_singular("analyses"));
    assert_eq!("codebasis", to_singular("codebases"));
    assert_eq!("diagnosis", to_singular("diagnoses"));
    assert_eq!("parenthesis", to_singular("parentheses"));
    assert_eq!("prognosis", to_singular("prognoses"));
    assert_eq!("synopsis", to_singular("synopses"));
    assert_eq!("thesis", to_singular("theses"));
    assert_eq!("knife", to_singular("knives"));
    assert_eq!("archive", to_singular("archives"));
    assert_eq!("motive", to_singular("motives"));
    assert_eq!("half", to_singular("halves"));
    assert_eq!("wolf", to_singular("wolves"));
    assert_eq!("calf", to_singular("calves"));
    assert_eq!("shelf", to_singular("shelves"));
    assert_eq!("series", to_singular("series"));
    assert_eq!("movie", to_singular("movies"));
    assert_eq!("bus", to_singular("buses"));
    assert_eq!("wish", to_singular("wishes"));
    assert_eq!("pitch", to_singular("pitches"));
    assert_eq!("box", to_singular("boxes"));
    assert_eq!("mouse", to_singular("mice"));
    assert_eq!("minibus", to_singular("minibuses"));
    assert_eq!("snowshoe", to_singular("snowshoes"));
    assert_eq!("crisis", to_singular("crises"));
    assert_eq!("ovotestis", to_singular("ovotestes"));
    assert_eq!("axis", to_singular("axes"));
    assert_eq!("octopus", to_singular("octopi"));
    assert_eq!("alias", to_singular("aliases"));
    assert_eq!("ox", to_singular("oxen"));
    assert_eq!("index", to_singular("indices"));
    assert_eq!("matrix", to_singular("matrices"));
    assert_eq!("quiz", to_singular("quizzes"));
    assert_eq!("database", to_singular("databases"));
}

#[test]
fn singularize_string_returns_none_option_if_no_match() {
    let expected_string: String = "bacon".to_owned();
    let asserted_string: String = to_singular("bacon");

    assert!(expected_string == asserted_string);
}
