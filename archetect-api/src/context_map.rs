use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The universal data interchange type for Archetect.
/// Replaces rhai::Map as the type for answers, settings, context data,
/// and archetype return values.
pub type ContextMap = BTreeMap<String, ContextValue>;

/// A dynamically-typed value that can be stored in a ContextMap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<ContextValue>),
    Map(ContextMap),
    Nil,
}

impl ContextValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ContextValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ContextValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&ContextMap> {
        match self {
            ContextValue::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<ContextValue>> {
        match self {
            ContextValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, ContextValue::Nil)
    }
}

impl From<String> for ContextValue {
    fn from(s: String) -> Self {
        ContextValue::String(s)
    }
}

impl From<&str> for ContextValue {
    fn from(s: &str) -> Self {
        ContextValue::String(s.to_string())
    }
}

impl From<i64> for ContextValue {
    fn from(i: i64) -> Self {
        ContextValue::Integer(i)
    }
}

impl From<f64> for ContextValue {
    fn from(f: f64) -> Self {
        ContextValue::Float(f)
    }
}

impl From<bool> for ContextValue {
    fn from(b: bool) -> Self {
        ContextValue::Boolean(b)
    }
}

impl From<Vec<ContextValue>> for ContextValue {
    fn from(a: Vec<ContextValue>) -> Self {
        ContextValue::Array(a)
    }
}

impl From<ContextMap> for ContextValue {
    fn from(m: ContextMap) -> Self {
        ContextValue::Map(m)
    }
}

impl From<Vec<String>> for ContextValue {
    fn from(v: Vec<String>) -> Self {
        ContextValue::Array(v.into_iter().map(ContextValue::String).collect())
    }
}

/// Convert from serde_json::Value to ContextValue.
impl From<serde_json::Value> for ContextValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => ContextValue::Nil,
            serde_json::Value::Bool(b) => ContextValue::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ContextValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    ContextValue::Float(f)
                } else {
                    ContextValue::Nil
                }
            }
            serde_json::Value::String(s) => ContextValue::String(s),
            serde_json::Value::Array(arr) => {
                ContextValue::Array(arr.into_iter().map(ContextValue::from).collect())
            }
            serde_json::Value::Object(obj) => {
                let map: ContextMap = obj
                    .into_iter()
                    .map(|(k, v)| (k, ContextValue::from(v)))
                    .collect();
                ContextValue::Map(map)
            }
        }
    }
}

/// Convert from ContextValue to serde_json::Value.
impl From<ContextValue> for serde_json::Value {
    fn from(v: ContextValue) -> Self {
        match v {
            ContextValue::Nil => serde_json::Value::Null,
            ContextValue::Boolean(b) => serde_json::Value::Bool(b),
            ContextValue::Integer(i) => serde_json::Value::Number(i.into()),
            ContextValue::Float(f) => {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            ContextValue::String(s) => serde_json::Value::String(s),
            ContextValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            ContextValue::Map(map) => {
                let obj: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect();
                serde_json::Value::Object(obj)
            }
        }
    }
}

impl std::fmt::Display for ContextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextValue::String(s) => write!(f, "{}", s),
            ContextValue::Integer(i) => write!(f, "{}", i),
            ContextValue::Float(fl) => write!(f, "{}", fl),
            ContextValue::Boolean(b) => write!(f, "{}", b),
            ContextValue::Nil => write!(f, ""),
            ContextValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            ContextValue::Map(_) => write!(f, "{{...}}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string() {
        let v: ContextValue = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn test_from_i64() {
        let v: ContextValue = 42i64.into();
        assert_eq!(v.as_i64(), Some(42));
    }

    #[test]
    fn test_from_bool() {
        let v: ContextValue = true.into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_json_roundtrip() {
        let mut map = ContextMap::new();
        map.insert("name".to_string(), "Jimmie".into());
        map.insert("age".to_string(), 42i64.into());
        map.insert("active".to_string(), true.into());
        map.insert("tags".to_string(), ContextValue::Array(vec![
            "rust".into(),
            "lua".into(),
        ]));

        let json = serde_json::to_string(&map).unwrap();
        let roundtripped: ContextMap = serde_json::from_str(&json).unwrap();
        assert_eq!(map, roundtripped);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let mut map = ContextMap::new();
        map.insert("name".to_string(), "Jimmie".into());
        map.insert("count".to_string(), 3i64.into());

        let yaml = serde_yaml::to_string(&map).unwrap();
        let roundtripped: ContextMap = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(map, roundtripped);
    }

    #[test]
    fn test_nested_map() {
        let mut inner = ContextMap::new();
        inner.insert("street".to_string(), "123 Main".into());

        let mut outer = ContextMap::new();
        outer.insert("address".to_string(), ContextValue::Map(inner));

        let json = serde_json::to_string(&outer).unwrap();
        let roundtripped: ContextMap = serde_json::from_str(&json).unwrap();
        assert_eq!(outer, roundtripped);
    }

    #[test]
    fn test_from_json_value() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test",
            "count": 5,
            "active": true,
            "tags": ["a", "b"]
        });

        let cv = ContextValue::from(json);
        match cv {
            ContextValue::Map(map) => {
                assert_eq!(map.get("name").unwrap().as_str(), Some("test"));
                assert_eq!(map.get("count").unwrap().as_i64(), Some(5));
                assert_eq!(map.get("active").unwrap().as_bool(), Some(true));
                assert!(map.get("tags").unwrap().as_array().is_some());
            }
            _ => panic!("Expected Map"),
        }
    }
}
