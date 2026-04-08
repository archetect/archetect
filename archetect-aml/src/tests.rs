#[cfg(test)]
mod tests {
    use crate::parse_yaml;
    use indoc::indoc;

    fn commerce_yaml() -> &'static str {
        indoc! {r#"
            spec: "1.0"
            organization: acme
            solution: commerce
            description: "E-commerce platform"

            types:
              Money:
                base: Decimal
                precision: 2
              OrderStatus:
                enum: [Draft, Submitted, Confirmed, Shipped, Delivered, Cancelled]

            entities:
              Customer:
                fields:
                  id: UUID
                  name: { type: String, required: true }
                  email: { type: String, required: true, unique: true }

              Product:
                fields:
                  id: UUID
                  name: { type: String, required: true }
                  sku: { type: String, required: true, unique: true }
                  price: { type: Money, required: true }
                  description: String

              Order:
                fields:
                  id: UUID
                  customer: { entity: Customer, relation: many-to-one, required: true }
                  status: { type: OrderStatus, default: Draft }
                  total: Money
                  created_at: { type: Timestamp, auto: create }
                events: [OrderCreated, OrderSubmitted, OrderCancelled]

              LineItem:
                fields:
                  id: UUID
                  order: { entity: Order, relation: many-to-one, required: true }
                  product: { entity: Product, relation: many-to-one, required: true }
                  quantity: { type: Integer, required: true }
                  price: Money

            boundaries:
              customer-service:
                type: service
                owns: [Customer]
              catalog-service:
                type: service
                owns: [Product]
              order-service:
                type: service
                owns: [Order, LineItem]

            interfaces:
              - from: order-service
                to: customer-service
                style: sync
              - from: order-service
                to: catalog-service
                style: sync
        "#}
    }

    #[test]
    fn test_parse_basic() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.organization(), "acme");
        assert_eq!(model.solution(), "commerce");
        assert_eq!(model.inner.description, "E-commerce platform");
    }

    #[test]
    fn test_entities_parsed() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.inner.entities.len(), 4);
        assert!(model.entity("Customer").is_some());
        assert!(model.entity("Order").is_some());
        assert!(model.entity("LineItem").is_some());
        assert!(model.entity("Product").is_some());
    }

    #[test]
    fn test_field_shorthand_normalization() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let customer = model.entity("Customer").unwrap();

        // id: UUID → key=true, type=UUID
        let id_field = customer.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(id_field.field_type.as_deref(), Some("UUID"));
        assert!(id_field.key);
        assert!(!id_field.is_relation);

        // name: { type: String, required: true }
        let name_field = customer.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name_field.field_type.as_deref(), Some("String"));
        assert!(name_field.required);

        // email: { type: String, required: true, unique: true }
        let email_field = customer.fields.iter().find(|f| f.name == "email").unwrap();
        assert!(email_field.unique);
        assert!(email_field.required);
    }

    #[test]
    fn test_relationship_fields() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let order = model.entity("Order").unwrap();

        let customer_field = order.fields.iter().find(|f| f.name == "customer").unwrap();
        assert!(customer_field.is_relation);
        assert_eq!(customer_field.entity.as_deref(), Some("Customer"));
        assert_eq!(customer_field.relation.as_deref(), Some("many-to-one"));
        assert!(customer_field.required);
    }

    #[test]
    fn test_entity_events() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let order = model.entity("Order").unwrap();
        assert_eq!(order.events, vec!["OrderCreated", "OrderSubmitted", "OrderCancelled"]);
    }

    #[test]
    fn test_boundaries() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.inner.boundaries.len(), 3);

        let order_svc = model.boundary("order-service").unwrap();
        assert_eq!(order_svc.boundary_type, "service");
        assert_eq!(order_svc.owns, vec!["Order", "LineItem"]);
    }

    #[test]
    fn test_entity_ownership() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.entity_owner("Customer"), Some("customer-service"));
        assert_eq!(model.entity_owner("Order"), Some("order-service"));
        assert_eq!(model.entity_owner("LineItem"), Some("order-service"));
        assert_eq!(model.entity_owner("Product"), Some("catalog-service"));
    }

    #[test]
    fn test_interfaces() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.inner.interfaces.len(), 2);

        let outbound = model.outbound_interfaces("order-service");
        assert_eq!(outbound.len(), 2);
        assert_eq!(outbound[0].to, "customer-service");
        assert_eq!(outbound[1].to, "catalog-service");
    }

    #[test]
    fn test_dependencies() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let deps = model.dependencies("order-service");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"customer-service".to_string()));
        assert!(deps.contains(&"catalog-service".to_string()));
    }

    #[test]
    fn test_remote_references() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let refs = model.remote_references("order-service");

        // Order.customer → Customer (owned by customer-service)
        // LineItem.product → Product (owned by catalog-service)
        assert!(refs.len() >= 2);

        let customer_ref = refs.iter().find(|r| r.target_entity == "Customer").unwrap();
        assert_eq!(customer_ref.source_entity, "Order");
        assert_eq!(customer_ref.target_boundary, "customer-service");

        let product_ref = refs.iter().find(|r| r.target_entity == "Product").unwrap();
        assert_eq!(product_ref.source_entity, "LineItem");
        assert_eq!(product_ref.target_boundary, "catalog-service");
    }

    #[test]
    fn test_expand_entity() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let customer = model.entity("Customer").unwrap();
        let expanded = model.expand_entity(customer);

        assert_eq!(expanded.name.pascal, "Customer");
        assert_eq!(expanded.name.snake, "customer");
        assert_eq!(expanded.name.kebab, "customer");

        assert!(!expanded.local_fields.is_empty());
        assert!(expanded.relations.is_empty()); // Customer has no relations
    }

    #[test]
    fn test_expand_entity_with_relations() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let order = model.entity("Order").unwrap();
        let expanded = model.expand_entity(order);

        assert!(!expanded.relations.is_empty());
        let customer_rel = expanded.relations.iter().find(|r| r.target_entity.as_deref() == Some("Customer")).unwrap();
        assert!(customer_rel.target.is_some());
        assert_eq!(customer_rel.target.as_ref().unwrap().pascal, "Customer");
    }

    #[test]
    fn test_entities_for_boundary() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let entities = model.entities_for("order-service");
        assert_eq!(entities.len(), 2);

        let names: Vec<&str> = entities.iter().map(|e| e.name.raw.as_str()).collect();
        assert!(names.contains(&"Order"));
        assert!(names.contains(&"LineItem"));
    }

    #[test]
    fn test_slice() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let slice = model.slice("order-service").unwrap();

        assert_eq!(slice.organization, "acme");
        assert_eq!(slice.solution, "commerce");
        assert_eq!(slice.boundary.name, "order-service");
        assert_eq!(slice.entities.len(), 2);
        assert_eq!(slice.outbound.len(), 2);
        assert_eq!(slice.dependencies.len(), 2);
        assert!(!slice.remote_references.is_empty());
    }

    #[test]
    fn test_org_solution() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        let os = model.org_solution();
        assert_eq!(os.raw, "acme-commerce");
        assert_eq!(os.kebab, "acme-commerce");
        assert_eq!(os.snake, "acme_commerce");
        assert_eq!(os.pascal, "AcmeCommerce");
    }

    #[test]
    fn test_types_parsed() {
        let model = parse_yaml(commerce_yaml()).unwrap();
        assert_eq!(model.inner.types.len(), 2);
        assert!(model.inner.types.contains_key("Money"));
        assert!(model.inner.types.contains_key("OrderStatus"));
    }

    #[test]
    fn test_implicit_id_field() {
        // Entity without explicit id should get one
        let yaml = indoc! {r#"
            entities:
              Thing:
                fields:
                  name: String
        "#};
        let model = parse_yaml(yaml).unwrap();
        let thing = model.entity("Thing").unwrap();
        let id_field = thing.fields.iter().find(|f| f.name == "id").unwrap();
        assert!(id_field.key);
        assert_eq!(id_field.field_type.as_deref(), Some("UUID"));
    }

    #[test]
    fn test_minimal_model() {
        let yaml = indoc! {r#"
            organization: test
            solution: minimal
            entities:
              Widget:
                fields:
                  id: UUID
                  label: String
            boundaries:
              widget-service:
                type: service
                owns: [Widget]
        "#};
        let model = parse_yaml(yaml).unwrap();
        assert_eq!(model.all_boundaries().len(), 1);
        assert_eq!(model.entities_for("widget-service").len(), 1);
    }
}
