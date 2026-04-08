use std::collections::BTreeMap;

use crate::model::ResolvedModel;
use crate::types::{AmlModel, Boundary, Entity, Interface, RawEntity, RawField};

/// Incrementally builds an AML model through programmatic or interactive construction.
#[derive(Debug, Clone)]
pub struct ModelBuilder {
    organization: String,
    solution: String,
    description: String,
    entities: BTreeMap<String, RawEntity>,
    boundaries: BTreeMap<String, Boundary>,
    interfaces: Vec<Interface>,
}

impl ModelBuilder {
    pub fn new() -> Self {
        ModelBuilder {
            organization: String::new(),
            solution: String::new(),
            description: String::new(),
            entities: BTreeMap::new(),
            boundaries: BTreeMap::new(),
            interfaces: Vec::new(),
        }
    }

    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = org.into();
        self
    }

    pub fn set_organization(&mut self, org: impl Into<String>) {
        self.organization = org.into();
    }

    pub fn solution(mut self, sol: impl Into<String>) -> Self {
        self.solution = sol.into();
        self
    }

    pub fn set_solution(&mut self, sol: impl Into<String>) {
        self.solution = sol.into();
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn set_description(&mut self, desc: impl Into<String>) {
        self.description = desc.into();
    }

    /// Add an entity with no fields (fields can be added later).
    pub fn add_entity(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.entities.entry(name).or_insert_with(|| RawEntity {
            fields: Some(BTreeMap::new()),
            events: None,
            operations: None,
            description: None,
        });
    }

    /// Add a field to an existing entity. Creates the entity if it doesn't exist.
    pub fn add_field(&mut self, entity_name: &str, field_name: impl Into<String>, raw: RawField) {
        let entity = self.entities.entry(entity_name.to_string()).or_insert_with(|| RawEntity {
            fields: Some(BTreeMap::new()),
            events: None,
            operations: None,
            description: None,
        });
        entity
            .fields
            .get_or_insert_with(BTreeMap::new)
            .insert(field_name.into(), raw);
    }

    /// Add a simple typed field (e.g., "name" → "String").
    pub fn add_simple_field(&mut self, entity_name: &str, field_name: impl Into<String>, field_type: impl Into<String>) {
        self.add_field(entity_name, field_name, RawField::Shorthand(field_type.into()));
    }

    /// Add a relationship field.
    pub fn add_relation_field(
        &mut self,
        entity_name: &str,
        field_name: impl Into<String>,
        target_entity: impl Into<String>,
        relation: impl Into<String>,
        required: bool,
    ) {
        self.add_field(
            entity_name,
            field_name,
            RawField::Full {
                field_type: None,
                entity: Some(target_entity.into()),
                relation: Some(relation.into()),
                required: Some(required),
                unique: None,
                key: None,
                default: None,
                auto: None,
                format: None,
                min: None,
                max: None,
            },
        );
    }

    /// Add a boundary.
    pub fn add_boundary(
        &mut self,
        name: impl Into<String>,
        boundary_type: impl Into<String>,
        owns: Vec<String>,
    ) {
        let name = name.into();
        self.boundaries.insert(
            name.clone(),
            Boundary {
                name: name.clone(),
                boundary_type: boundary_type.into(),
                owns,
                language: None,
                persistence: None,
                external: None,
                style: None,
                description: None,
            },
        );
    }

    /// Add an interface between two boundaries.
    pub fn add_interface(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        style: impl Into<String>,
    ) {
        self.interfaces.push(Interface {
            from: from.into(),
            to: to.into(),
            style: style.into(),
            operations: Vec::new(),
            events: Vec::new(),
            commands: Vec::new(),
            expose: Vec::new(),
        });
    }

    /// Build the model, normalizing all entities and resolving indices.
    pub fn build(self) -> ResolvedModel {
        // Normalize entities from raw
        let mut entities = BTreeMap::new();
        for (name, raw) in &self.entities {
            entities.insert(name.clone(), Entity::from_raw(name, raw));
        }

        let model = AmlModel {
            spec: "1.0".to_string(),
            organization: self.organization,
            solution: self.solution,
            description: self.description,
            types: BTreeMap::new(),
            entities,
            boundaries: self.boundaries,
            interfaces: self.interfaces,
        };

        ResolvedModel::from_model(model)
    }
}

impl Default for ModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut builder = ModelBuilder::new()
            .organization("acme")
            .solution("test");

        builder.add_entity("Customer");
        builder.add_simple_field("Customer", "name", "String");
        builder.add_simple_field("Customer", "email", "String");

        builder.add_entity("Order");
        builder.add_simple_field("Order", "total", "Decimal");
        builder.add_relation_field("Order", "customer", "Customer", "many-to-one", true);

        builder.add_boundary("customer-service", "service", vec!["Customer".to_string()]);
        builder.add_boundary("order-service", "service", vec!["Order".to_string()]);
        builder.add_interface("order-service", "customer-service", "sync");

        let model = builder.build();

        assert_eq!(model.organization(), "acme");
        assert_eq!(model.solution(), "test");
        assert_eq!(model.all_boundaries().len(), 2);

        let customer = model.entity("Customer").unwrap();
        assert!(customer.fields.iter().any(|f| f.name == "id" && f.key));
        assert!(customer.fields.iter().any(|f| f.name == "name"));

        let order = model.entity("Order").unwrap();
        let customer_rel = order.fields.iter().find(|f| f.name == "customer").unwrap();
        assert!(customer_rel.is_relation);
        assert_eq!(customer_rel.entity.as_deref(), Some("Customer"));

        assert_eq!(model.dependencies("order-service"), vec!["customer-service"]);
        assert_eq!(model.entity_owner("Customer"), Some("customer-service"));

        let slice = model.slice("order-service").unwrap();
        assert_eq!(slice.entities.len(), 1);
        assert!(!slice.remote_references.is_empty());
    }

    #[test]
    fn test_builder_implicit_id() {
        let mut builder = ModelBuilder::new();
        builder.add_entity("Widget");
        builder.add_simple_field("Widget", "label", "String");

        let model = builder.build();
        let widget = model.entity("Widget").unwrap();
        assert!(widget.fields.iter().any(|f| f.name == "id" && f.key));
    }
}
