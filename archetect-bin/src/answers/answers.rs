use archetect_api::{ContextMap, ContextValue};
use archetect_core::errors::{AnswerFileError, ArchetectError};
use camino::Utf8Path;
use std::fs;

pub fn read_answers<P: AsRef<Utf8Path>>(path: P) -> Result<ContextMap, ArchetectError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::MissingError,
        });
    }
    match path.extension() {
        Some("yml") | Some("yaml") => read_yaml_answers(path),
        Some("json") => read_json_answers(path),
        Some("rhai") => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::ParseError(
                "Rhai answer files (.rhai) are not supported. Use YAML or JSON instead, \
                 or use `archetect2` for legacy archetypes."
                    .to_string(),
            ),
        }),
        _ => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidFileType,
        }),
    }
}

fn read_yaml_answers(path: &Utf8Path) -> Result<ContextMap, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    serde_yaml::from_str(&contents).map_err(|err| ArchetectError::AnswerConfigError {
        path: path.to_string(),
        source: AnswerFileError::ParseError(err.to_string()),
    })
}

fn read_json_answers(path: &Utf8Path) -> Result<ContextMap, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(|err| ArchetectError::AnswerConfigError {
        path: path.to_string(),
        source: AnswerFileError::ParseError(err.to_string()),
    })
}

/// Split a `-a key=value` argument into its key and raw value string.
///
/// Splits on the first `=`. The key must be non-empty. The value may be empty.
/// Keys may contain dots for nested map access (e.g. `db.host`).
pub fn parse_answer_pair(input: &str) -> Result<(String, String), anyhow::Error> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("expected KEY=VALUE, got '{}'", input))?;

    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow::anyhow!("key must not be empty in '{}'", input));
    }

    Ok((key.to_owned(), value.to_owned()))
}

/// Parse a raw value string as YAML, falling back to a plain string if YAML
/// parsing fails or produces an unexpected type.
///
/// This gives CLI answers the same type semantics as YAML answer files:
/// - `42` → Integer
/// - `1.5` → Float
/// - `true` / `false` → Boolean
/// - `null` → Nil
/// - `[a, b, c]` → Array
/// - `{host: localhost, port: 5432}` → Map
/// - `"42"` → String (YAML quoted string)
/// - `hello world` → String
pub fn parse_answer_value(raw: &str) -> ContextValue {
    // Empty string stays as empty string — don't let YAML parse it as null.
    if raw.is_empty() {
        return ContextValue::String(String::new());
    }

    match serde_yaml::from_str::<ContextValue>(raw) {
        Ok(value) => value,
        Err(_) => ContextValue::String(raw.to_owned()),
    }
}

/// Insert a value into a ContextMap, supporting dotted keys for nested maps.
///
/// `db.host` with value `localhost` produces `{db: {host: "localhost"}}`.
/// Intermediate keys are created as maps if they don't exist.
/// If an intermediate key exists but isn't a map, it is overwritten with a map.
pub fn insert_dotted(map: &mut ContextMap, key: &str, value: ContextValue) {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        map.insert(key.to_owned(), value);
        return;
    }

    let mut current = map;
    for part in &parts[..parts.len() - 1] {
        // Navigate into or create intermediate maps.
        let entry = current
            .entry((*part).to_owned())
            .or_insert_with(|| ContextValue::Map(ContextMap::new()));

        // If the existing entry isn't a map, replace it with one.
        if !matches!(entry, ContextValue::Map(_)) {
            *entry = ContextValue::Map(ContextMap::new());
        }

        current = match entry {
            ContextValue::Map(m) => m,
            _ => unreachable!(),
        };
    }

    let leaf = parts.last().expect("parts is non-empty");
    current.insert((*leaf).to_owned(), value);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_answer_pair ──────────────────────────────────────────

    #[test]
    fn test_simple_pair() {
        let (k, v) = parse_answer_pair("name=hello").unwrap();
        assert_eq!(k, "name");
        assert_eq!(v, "hello");
    }

    #[test]
    fn test_equals_in_value() {
        let (k, v) = parse_answer_pair("expr=a=b=c").unwrap();
        assert_eq!(k, "expr");
        assert_eq!(v, "a=b=c");
    }

    #[test]
    fn test_empty_value() {
        let (k, v) = parse_answer_pair("key=").unwrap();
        assert_eq!(k, "key");
        assert_eq!(v, "");
    }

    #[test]
    fn test_dotted_key_pair() {
        let (k, v) = parse_answer_pair("db.host=localhost").unwrap();
        assert_eq!(k, "db.host");
        assert_eq!(v, "localhost");
    }

    #[test]
    fn test_missing_equals() {
        assert!(parse_answer_pair("noequals").is_err());
    }

    #[test]
    fn test_empty_key() {
        assert!(parse_answer_pair("=value").is_err());
    }

    #[test]
    fn test_whitespace_around_key() {
        let (k, v) = parse_answer_pair("  name  =hello").unwrap();
        assert_eq!(k, "name");
        assert_eq!(v, "hello");
    }

    // ── parse_answer_value ─────────────────────────────────────────

    #[test]
    fn test_value_plain_string() {
        assert_eq!(
            parse_answer_value("hello"),
            ContextValue::String("hello".to_owned())
        );
    }

    #[test]
    fn test_value_string_with_spaces() {
        assert_eq!(
            parse_answer_value("hello world"),
            ContextValue::String("hello world".to_owned())
        );
    }

    #[test]
    fn test_value_integer() {
        assert_eq!(parse_answer_value("42"), ContextValue::Integer(42));
    }

    #[test]
    fn test_value_negative_integer() {
        assert_eq!(parse_answer_value("-7"), ContextValue::Integer(-7));
    }

    #[test]
    fn test_value_float() {
        assert_eq!(parse_answer_value("1.5"), ContextValue::Float(1.5));
    }

    #[test]
    fn test_value_bool_true() {
        assert_eq!(parse_answer_value("true"), ContextValue::Boolean(true));
    }

    #[test]
    fn test_value_bool_false() {
        assert_eq!(parse_answer_value("false"), ContextValue::Boolean(false));
    }

    #[test]
    fn test_value_null() {
        assert_eq!(parse_answer_value("null"), ContextValue::Nil);
    }

    #[test]
    fn test_value_empty_string() {
        assert_eq!(
            parse_answer_value(""),
            ContextValue::String(String::new())
        );
    }

    #[test]
    fn test_value_quoted_integer_stays_string() {
        // YAML: "42" is a quoted string, not an integer
        assert_eq!(
            parse_answer_value("\"42\""),
            ContextValue::String("42".to_owned())
        );
    }

    #[test]
    fn test_value_quoted_string() {
        assert_eq!(
            parse_answer_value("\"hello\""),
            ContextValue::String("hello".to_owned())
        );
    }

    #[test]
    fn test_value_phone_number_is_integer() {
        // Without quoting, all-digit strings become integers — this is correct
        // YAML behavior. Users who want a string should quote: "5551234"
        assert_eq!(parse_answer_value("5551234"), ContextValue::Integer(5551234));
    }

    #[test]
    fn test_value_quoted_phone_number_stays_string() {
        assert_eq!(
            parse_answer_value("\"5551234\""),
            ContextValue::String("5551234".to_owned())
        );
    }

    #[test]
    fn test_value_array() {
        let result = parse_answer_value("[a, b, c]");
        match result {
            ContextValue::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], ContextValue::String("a".to_owned()));
                assert_eq!(arr[1], ContextValue::String("b".to_owned()));
                assert_eq!(arr[2], ContextValue::String("c".to_owned()));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_value_mixed_array() {
        let result = parse_answer_value("[hello, 42, true]");
        match result {
            ContextValue::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], ContextValue::String("hello".to_owned()));
                assert_eq!(arr[1], ContextValue::Integer(42));
                assert_eq!(arr[2], ContextValue::Boolean(true));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_value_map() {
        let result = parse_answer_value("{host: localhost, port: 5432}");
        match result {
            ContextValue::Map(map) => {
                assert_eq!(
                    map.get("host"),
                    Some(&ContextValue::String("localhost".to_owned()))
                );
                assert_eq!(map.get("port"), Some(&ContextValue::Integer(5432)));
            }
            other => panic!("Expected Map, got {:?}", other),
        }
    }

    // ── insert_dotted ──────────────────────────────────────────────

    #[test]
    fn test_insert_simple_key() {
        let mut map = ContextMap::new();
        insert_dotted(&mut map, "name", ContextValue::String("hello".to_owned()));
        assert_eq!(
            map.get("name"),
            Some(&ContextValue::String("hello".to_owned()))
        );
    }

    #[test]
    fn test_insert_dotted_key() {
        let mut map = ContextMap::new();
        insert_dotted(
            &mut map,
            "db.host",
            ContextValue::String("localhost".to_owned()),
        );
        insert_dotted(&mut map, "db.port", ContextValue::Integer(5432));

        match map.get("db") {
            Some(ContextValue::Map(db)) => {
                assert_eq!(
                    db.get("host"),
                    Some(&ContextValue::String("localhost".to_owned()))
                );
                assert_eq!(db.get("port"), Some(&ContextValue::Integer(5432)));
            }
            other => panic!("Expected Map at 'db', got {:?}", other),
        }
    }

    #[test]
    fn test_insert_deeply_nested() {
        let mut map = ContextMap::new();
        insert_dotted(
            &mut map,
            "a.b.c.d",
            ContextValue::String("deep".to_owned()),
        );

        let a = map.get("a").unwrap().as_map().unwrap();
        let b = a.get("b").unwrap().as_map().unwrap();
        let c = b.get("c").unwrap().as_map().unwrap();
        assert_eq!(c.get("d"), Some(&ContextValue::String("deep".to_owned())));
    }

    #[test]
    fn test_insert_dotted_overwrites_non_map() {
        let mut map = ContextMap::new();
        map.insert("db".to_owned(), ContextValue::String("old".to_owned()));

        insert_dotted(
            &mut map,
            "db.host",
            ContextValue::String("localhost".to_owned()),
        );

        match map.get("db") {
            Some(ContextValue::Map(db)) => {
                assert_eq!(
                    db.get("host"),
                    Some(&ContextValue::String("localhost".to_owned()))
                );
            }
            other => panic!("Expected Map at 'db', got {:?}", other),
        }
    }

    #[test]
    fn test_insert_dotted_preserves_siblings() {
        let mut map = ContextMap::new();
        insert_dotted(
            &mut map,
            "db.host",
            ContextValue::String("localhost".to_owned()),
        );
        insert_dotted(&mut map, "db.port", ContextValue::Integer(5432));
        insert_dotted(
            &mut map,
            "app.name",
            ContextValue::String("myapp".to_owned()),
        );

        let db = map.get("db").unwrap().as_map().unwrap();
        assert_eq!(
            db.get("host"),
            Some(&ContextValue::String("localhost".to_owned()))
        );
        assert_eq!(db.get("port"), Some(&ContextValue::Integer(5432)));

        let app = map.get("app").unwrap().as_map().unwrap();
        assert_eq!(
            app.get("name"),
            Some(&ContextValue::String("myapp".to_owned()))
        );
    }
}
