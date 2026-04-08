use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// A complete AML model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmlModel {
    #[serde(default = "default_spec")]
    pub spec: String,
    #[serde(default)]
    pub organization: String,
    #[serde(default)]
    pub solution: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub types: BTreeMap<String, TypeDef>,
    #[serde(default, deserialize_with = "deserialize_entities")]
    pub entities: BTreeMap<String, Entity>,
    #[serde(default)]
    pub boundaries: BTreeMap<String, Boundary>,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
}

fn default_spec() -> String {
    "1.0".to_string()
}

/// A reusable type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeDef {
    Enum {
        #[serde(rename = "enum")]
        variants: Vec<String>,
        #[serde(default)]
        description: Option<String>,
    },
    Value {
        value: BTreeMap<String, RawField>,
        #[serde(default)]
        description: Option<String>,
    },
    Base {
        base: String,
        #[serde(default)]
        precision: Option<u32>,
        #[serde(default)]
        description: Option<String>,
    },
}

/// A domain entity with normalized fields.
#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    pub name: String,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// A normalized field.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub key: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub auto: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    // Relationship fields
    #[serde(default)]
    pub is_relation: bool,
    #[serde(default)]
    pub entity: Option<String>,
    #[serde(default)]
    pub relation: Option<String>,
}

/// A service/deployment boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boundary {
    #[serde(skip_deserializing)]
    pub name: String,
    #[serde(rename = "type", default = "default_boundary_type")]
    pub boundary_type: String,
    #[serde(default)]
    pub owns: Vec<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub persistence: Option<serde_yaml::Value>,
    #[serde(default)]
    pub external: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_boundary_type() -> String {
    "service".to_string()
}

/// An interface between two boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub from: String,
    pub to: String,
    #[serde(default = "default_interface_style")]
    pub style: String,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub expose: Vec<String>,
}

fn default_interface_style() -> String {
    "sync".to_string()
}

// ── Raw deserialization types for field shorthand ────────────────

/// Raw field as it appears in YAML — either a type string or a full object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawField {
    Shorthand(String),
    Full {
        #[serde(rename = "type")]
        field_type: Option<String>,
        #[serde(default)]
        entity: Option<String>,
        #[serde(default)]
        relation: Option<String>,
        #[serde(default)]
        required: Option<bool>,
        #[serde(default)]
        unique: Option<bool>,
        #[serde(default)]
        key: Option<bool>,
        #[serde(default)]
        default: Option<String>,
        #[serde(default)]
        auto: Option<String>,
        #[serde(default)]
        format: Option<String>,
        #[serde(default)]
        min: Option<i64>,
        #[serde(default)]
        max: Option<i64>,
    },
}

/// Raw entity as it appears in YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct RawEntity {
    #[serde(default)]
    pub fields: Option<BTreeMap<String, RawField>>,
    #[serde(default)]
    pub events: Option<Vec<String>>,
    #[serde(default)]
    pub operations: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
}

// ── Normalization ───────────────────────────────────────────────

impl Field {
    /// Normalize a raw field definition into a full Field.
    pub fn from_raw(name: &str, raw: &RawField) -> Field {
        match raw {
            RawField::Shorthand(type_name) => {
                let is_id = name == "id";
                Field {
                    name: name.to_string(),
                    field_type: Some(if is_id && type_name == "UUID" {
                        "UUID".to_string()
                    } else {
                        type_name.clone()
                    }),
                    required: false,
                    unique: false,
                    key: is_id,
                    default: None,
                    auto: if is_id { Some("create".to_string()) } else { None },
                    format: None,
                    is_relation: false,
                    entity: None,
                    relation: None,
                }
            }
            RawField::Full {
                field_type,
                entity,
                relation,
                required,
                unique,
                key,
                default,
                auto,
                format,
                ..
            } => {
                let is_id = name == "id";
                let is_relation = entity.is_some();
                Field {
                    name: name.to_string(),
                    field_type: if is_id && field_type.is_none() {
                        Some("UUID".to_string())
                    } else {
                        field_type.clone()
                    },
                    required: required.unwrap_or(false),
                    unique: unique.unwrap_or(false),
                    key: key.unwrap_or(is_id),
                    default: default.clone(),
                    auto: auto.clone().or_else(|| {
                        if is_id { Some("create".to_string()) } else { None }
                    }),
                    format: format.clone(),
                    is_relation,
                    entity: entity.clone(),
                    relation: if is_relation {
                        Some(relation.clone().unwrap_or_else(|| "many-to-one".to_string()))
                    } else {
                        None
                    },
                }
            }
        }
    }
}

impl Entity {
    /// Normalize a raw entity definition.
    pub fn from_raw(name: &str, raw: &RawEntity) -> Entity {
        let mut fields = Vec::new();
        let raw_fields = raw.fields.clone().unwrap_or_default();

        // Check if there's an explicit id field
        let has_id = raw_fields.contains_key("id");

        // Add implicit id if not present
        if !has_id {
            fields.push(Field {
                name: "id".to_string(),
                field_type: Some("UUID".to_string()),
                required: false,
                unique: false,
                key: true,
                default: None,
                auto: Some("create".to_string()),
                format: None,
                is_relation: false,
                entity: None,
                relation: None,
            });
        }

        // Normalize each field (sorted for stable output)
        for (fname, fraw) in &raw_fields {
            fields.push(Field::from_raw(fname, fraw));
        }

        Entity {
            name: name.to_string(),
            fields,
            events: raw.events.clone().unwrap_or_default(),
            operations: raw.operations.clone().unwrap_or_default(),
            description: raw.description.clone(),
        }
    }
}

/// Custom deserializer for entities that normalizes from RawEntity.
fn deserialize_entities<'de, D>(deserializer: D) -> Result<BTreeMap<String, Entity>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_map: BTreeMap<String, RawEntity> = BTreeMap::deserialize(deserializer)?;
    let mut entities = BTreeMap::new();
    for (name, raw) in &raw_map {
        entities.insert(name.clone(), Entity::from_raw(name, raw));
    }
    Ok(entities)
}

impl AmlModel {
    /// Post-deserialization: inject boundary names from map keys.
    pub fn resolve_names(&mut self) {
        for (name, boundary) in &mut self.boundaries {
            boundary.name = name.clone();
        }
    }
}
