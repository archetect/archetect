local model = require("archetect.model")

local context = Context.new()

-- Test: model.builder() API
local builder = model.builder()
builder:set_organization("test-org")
builder:set_solution("test-sol")
builder:add_entity("Widget")
builder:add_field("Widget", "name", "String")
builder:add_field("Widget", "price", "Decimal")
builder:add_entity("Order")
builder:add_field("Order", "total", "Decimal")
builder:add_relation("Order", "widget", "Widget", "many-to-one", true)
builder:add_boundary("widget-service", "service", {"Widget"})
builder:add_boundary("order-service", "service", {"Order"})
builder:add_interface("order-service", "widget-service", "sync")

local m = builder:build()

-- Verify model identity
log.info("org=" .. m:organization())
log.info("sol=" .. m:solution())

local os = m:org_solution()
log.info("org_solution.kebab=" .. os.kebab)
log.info("org_solution.pascal=" .. os.pascal)

-- Verify entities_for
local entities = m:entities_for("order-service")
log.info("order-service entities=" .. #entities)
log.info("entity_name=" .. entities[1].name.pascal)

-- Verify slice
local slice = m:slice("widget-service")
log.info("slice.boundary=" .. slice.boundary.name)
log.info("slice.entities=" .. #slice.entities)

-- Verify dependencies
local deps = m:dependencies("order-service")
log.info("deps=" .. #deps)
log.info("dep=" .. deps[1])

-- Verify remote references
local refs = m:remote_references("order-service")
log.info("remote_refs=" .. #refs)
log.info("ref_target=" .. refs[1].target_entity)

return context
