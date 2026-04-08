use archetect_api::{ContextMap, ContextValue};
use rhai::{Dynamic, Map};

/// Convert a `rhai::Map` to a `ContextMap`.
pub fn rhai_map_to_context_map(map: &Map) -> ContextMap {
    map.iter()
        .map(|(k, v)| (k.to_string(), rhai_dynamic_to_context_value(v)))
        .collect()
}

/// Convert a `ContextMap` to a `rhai::Map`.
pub fn context_map_to_rhai_map(map: &ContextMap) -> Map {
    map.iter()
        .map(|(k, v)| (k.clone().into(), context_value_to_rhai_dynamic(v)))
        .collect()
}

/// Convert a `rhai::Dynamic` to a `ContextValue`.
pub fn rhai_dynamic_to_context_value(d: &Dynamic) -> ContextValue {
    if d.is_unit() {
        ContextValue::Nil
    } else if let Some(s) = d.clone().try_cast::<String>() {
        ContextValue::String(s)
    } else if let Some(i) = d.clone().try_cast::<i64>() {
        ContextValue::Integer(i)
    } else if let Some(b) = d.clone().try_cast::<bool>() {
        ContextValue::Boolean(b)
    } else if let Some(f) = d.clone().try_cast::<f64>() {
        ContextValue::Float(f)
    } else if let Some(arr) = d.clone().try_cast::<Vec<Dynamic>>() {
        ContextValue::Array(arr.iter().map(rhai_dynamic_to_context_value).collect())
    } else if let Some(map) = d.clone().try_cast::<Map>() {
        ContextValue::Map(rhai_map_to_context_map(&map))
    } else {
        // Fallback: stringify unknown types
        ContextValue::String(d.to_string())
    }
}

/// Convert a `ContextValue` to a `rhai::Dynamic`.
pub fn context_value_to_rhai_dynamic(v: &ContextValue) -> Dynamic {
    match v {
        ContextValue::Nil => Dynamic::UNIT,
        ContextValue::String(s) => Dynamic::from(s.clone()),
        ContextValue::Integer(i) => Dynamic::from(*i),
        ContextValue::Float(f) => Dynamic::from(*f),
        ContextValue::Boolean(b) => Dynamic::from(*b),
        ContextValue::Array(arr) => {
            let rhai_arr: Vec<Dynamic> = arr.iter().map(context_value_to_rhai_dynamic).collect();
            Dynamic::from(rhai_arr)
        }
        ContextValue::Map(map) => Dynamic::from(context_map_to_rhai_map(map)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_simple_map() {
        let mut map = ContextMap::new();
        map.insert("name".to_string(), ContextValue::String("Jimmie".to_string()));
        map.insert("age".to_string(), ContextValue::Integer(42));
        map.insert("active".to_string(), ContextValue::Boolean(true));

        let rhai_map = context_map_to_rhai_map(&map);
        let roundtripped = rhai_map_to_context_map(&rhai_map);
        assert_eq!(map, roundtripped);
    }

    #[test]
    fn test_roundtrip_nested() {
        let mut inner = ContextMap::new();
        inner.insert("street".to_string(), ContextValue::String("123 Main".to_string()));

        let mut map = ContextMap::new();
        map.insert("address".to_string(), ContextValue::Map(inner));
        map.insert("tags".to_string(), ContextValue::Array(vec![
            ContextValue::String("rust".to_string()),
            ContextValue::String("lua".to_string()),
        ]));

        let rhai_map = context_map_to_rhai_map(&map);
        let roundtripped = rhai_map_to_context_map(&rhai_map);
        assert_eq!(map, roundtripped);
    }

    #[test]
    fn test_rhai_dynamic_string() {
        let d = Dynamic::from("hello".to_string());
        let cv = rhai_dynamic_to_context_value(&d);
        assert_eq!(cv, ContextValue::String("hello".to_string()));
    }

    #[test]
    fn test_rhai_dynamic_unit() {
        let d = Dynamic::UNIT;
        let cv = rhai_dynamic_to_context_value(&d);
        assert_eq!(cv, ContextValue::Nil);
    }

    #[test]
    fn test_rhai_dynamic_array() {
        let arr: Vec<Dynamic> = vec![
            Dynamic::from("a".to_string()),
            Dynamic::from("b".to_string()),
        ];
        let d = Dynamic::from(arr);
        let cv = rhai_dynamic_to_context_value(&d);
        assert_eq!(cv, ContextValue::Array(vec![
            ContextValue::String("a".to_string()),
            ContextValue::String("b".to_string()),
        ]));
    }
}
