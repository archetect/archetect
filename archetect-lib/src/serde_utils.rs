use serde::{Deserialize, Deserializer };
use linked_hash_map::LinkedHashMap;
use std::hash::Hash;

pub fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

pub fn deserialize_optional_map<'de, K, V, D>(deserializer: D) -> Result<LinkedHashMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
{
    LinkedHashMap::deserialize(deserializer).or(Ok(LinkedHashMap::new()))
}