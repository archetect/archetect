use std::collections::BTreeMap;

use crate::types::{AmlModel, Boundary, Entity, Interface};

/// A resolved AML model with precomputed indices for fast queries.
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub inner: AmlModel,
    /// Entity name → boundary name that owns it
    entity_owners: BTreeMap<String, String>,
    /// Boundary name → outbound interfaces
    outbound: BTreeMap<String, Vec<Interface>>,
    /// Boundary name → inbound interfaces
    inbound: BTreeMap<String, Vec<Interface>>,
}

/// A model slice for a single boundary — everything an archetype needs.
#[derive(Debug, Clone)]
pub struct BoundarySlice {
    pub organization: String,
    pub solution: String,
    pub boundary: Boundary,
    pub entities: Vec<ExpandedEntity>,
    pub remote_references: Vec<RemoteReference>,
    pub outbound: Vec<Interface>,
    pub inbound: Vec<Interface>,
    pub dependencies: Vec<String>,
    pub types: BTreeMap<String, crate::types::TypeDef>,
}

/// An entity with case-expanded names on the entity and all fields.
#[derive(Debug, Clone)]
pub struct ExpandedEntity {
    pub name: CaseVariants,
    pub fields: Vec<ExpandedField>,
    pub local_fields: Vec<ExpandedField>,
    pub relations: Vec<ExpandedField>,
    pub events: Vec<String>,
    pub operations: Vec<String>,
}

/// A field with case-expanded name.
#[derive(Debug, Clone)]
pub struct ExpandedField {
    pub name: CaseVariants,
    pub field_type: Option<String>,
    pub required: bool,
    pub unique: bool,
    pub key: bool,
    pub default: Option<String>,
    pub is_relation: bool,
    pub relation: Option<String>,
    pub target_entity: Option<String>,
    pub target: Option<CaseVariants>,
}

/// All standard programming case variants of a name.
#[derive(Debug, Clone)]
pub struct CaseVariants {
    pub raw: String,
    pub snake: String,
    pub pascal: String,
    pub camel: String,
    pub kebab: String,
    pub train: String,
    pub constant: String,
    pub title: String,
}

/// A relationship that crosses boundary lines.
#[derive(Debug, Clone)]
pub struct RemoteReference {
    pub source_entity: String,
    pub field_name: String,
    pub target_entity: String,
    pub target_boundary: String,
    pub relation: String,
}

impl CaseVariants {
    pub fn from_name(name: &str) -> Self {
        CaseVariants {
            raw: name.to_string(),
            snake: archetect_inflections::to_snake_case(name),
            pascal: archetect_inflections::to_pascal_case(name),
            camel: archetect_inflections::to_camel_case(name),
            kebab: archetect_inflections::to_kebab_case(name),
            train: archetect_inflections::to_train_case(name),
            constant: archetect_inflections::to_screaming_snake_case(name),
            title: archetect_inflections::to_title_case(name),
        }
    }
}

impl ResolvedModel {
    /// Resolve a parsed AmlModel into a queryable ResolvedModel.
    pub fn from_model(mut model: AmlModel) -> Self {
        model.resolve_names();

        // Build entity ownership index
        let mut entity_owners = BTreeMap::new();
        for (bname, boundary) in &model.boundaries {
            for ename in &boundary.owns {
                entity_owners.insert(ename.clone(), bname.clone());
            }
        }

        // Build interface indices
        let mut outbound: BTreeMap<String, Vec<Interface>> = BTreeMap::new();
        let mut inbound: BTreeMap<String, Vec<Interface>> = BTreeMap::new();
        for iface in &model.interfaces {
            outbound
                .entry(iface.from.clone())
                .or_default()
                .push(iface.clone());
            inbound
                .entry(iface.to.clone())
                .or_default()
                .push(iface.clone());
        }

        ResolvedModel {
            inner: model,
            entity_owners,
            outbound,
            inbound,
        }
    }

    // ── Accessors ───────────────────────────────────────────────

    pub fn organization(&self) -> &str {
        &self.inner.organization
    }

    pub fn solution(&self) -> &str {
        &self.inner.solution
    }

    pub fn entity(&self, name: &str) -> Option<&Entity> {
        self.inner.entities.get(name)
    }

    pub fn boundary(&self, name: &str) -> Option<&Boundary> {
        self.inner.boundaries.get(name)
    }

    pub fn all_boundaries(&self) -> Vec<&Boundary> {
        self.inner.boundaries.values().collect()
    }

    pub fn boundaries_of_type(&self, boundary_type: &str) -> Vec<&Boundary> {
        self.inner
            .boundaries
            .values()
            .filter(|b| b.boundary_type == boundary_type)
            .collect()
    }

    pub fn outbound_interfaces(&self, boundary_name: &str) -> &[Interface] {
        self.outbound
            .get(boundary_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn inbound_interfaces(&self, boundary_name: &str) -> &[Interface] {
        self.inbound
            .get(boundary_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn dependencies(&self, boundary_name: &str) -> Vec<String> {
        let mut deps = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for iface in self.outbound_interfaces(boundary_name) {
            if seen.insert(iface.to.clone()) {
                deps.push(iface.to.clone());
            }
        }
        deps
    }

    pub fn entity_owner(&self, entity_name: &str) -> Option<&str> {
        self.entity_owners.get(entity_name).map(|s| s.as_str())
    }

    // ── Expansion ───────────────────────────────────────────────

    /// Expand an entity with case variants on entity name and all fields.
    pub fn expand_entity(&self, entity: &Entity) -> ExpandedEntity {
        let mut fields = Vec::new();
        let mut local_fields = Vec::new();
        let mut relations = Vec::new();

        for field in &entity.fields {
            let expanded = ExpandedField {
                name: CaseVariants::from_name(&field.name),
                field_type: field.field_type.clone(),
                required: field.required,
                unique: field.unique,
                key: field.key,
                default: field.default.clone(),
                is_relation: field.is_relation,
                relation: field.relation.clone(),
                target_entity: field.entity.clone(),
                target: field.entity.as_ref().map(|e| CaseVariants::from_name(e)),
            };

            fields.push(expanded.clone());
            if field.is_relation {
                relations.push(expanded);
            } else {
                local_fields.push(expanded);
            }
        }

        ExpandedEntity {
            name: CaseVariants::from_name(&entity.name),
            fields,
            local_fields,
            relations,
            events: entity.events.clone(),
            operations: entity.operations.clone(),
        }
    }

    /// Get fully expanded entities owned by a boundary.
    pub fn entities_for(&self, boundary_name: &str) -> Vec<ExpandedEntity> {
        let boundary = match self.inner.boundaries.get(boundary_name) {
            Some(b) => b,
            None => return vec![],
        };
        boundary
            .owns
            .iter()
            .filter_map(|ename| {
                self.inner.entities.get(ename).map(|e| self.expand_entity(e))
            })
            .collect()
    }

    /// Find entity relations that cross boundary lines.
    pub fn remote_references(&self, boundary_name: &str) -> Vec<RemoteReference> {
        let boundary = match self.inner.boundaries.get(boundary_name) {
            Some(b) => b,
            None => return vec![],
        };

        let mut result = Vec::new();
        for ename in &boundary.owns {
            if let Some(entity) = self.inner.entities.get(ename) {
                for field in &entity.fields {
                    if field.is_relation {
                        if let Some(target) = &field.entity {
                            if let Some(owner) = self.entity_owners.get(target) {
                                if owner != boundary_name {
                                    result.push(RemoteReference {
                                        source_entity: ename.clone(),
                                        field_name: field.name.clone(),
                                        target_entity: target.clone(),
                                        target_boundary: owner.clone(),
                                        relation: field
                                            .relation
                                            .clone()
                                            .unwrap_or_else(|| "many-to-one".to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }

    // ── Slicing ─────────────────────────────────────────────────

    /// Build a complete model slice for a boundary.
    pub fn slice(&self, boundary_name: &str) -> Option<BoundarySlice> {
        let boundary = self.inner.boundaries.get(boundary_name)?;
        Some(BoundarySlice {
            organization: self.inner.organization.clone(),
            solution: self.inner.solution.clone(),
            boundary: boundary.clone(),
            entities: self.entities_for(boundary_name),
            remote_references: self.remote_references(boundary_name),
            outbound: self.outbound_interfaces(boundary_name).to_vec(),
            inbound: self.inbound_interfaces(boundary_name).to_vec(),
            dependencies: self.dependencies(boundary_name),
            types: self.inner.types.clone(),
        })
    }

    /// Get org-solution compound name with case variants.
    pub fn org_solution(&self) -> CaseVariants {
        let compound = format!("{}-{}", self.inner.organization, self.inner.solution);
        CaseVariants::from_name(&compound)
    }
}
