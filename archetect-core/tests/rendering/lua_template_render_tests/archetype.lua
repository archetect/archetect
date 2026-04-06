local ctx = Context.new()

ctx:text("Name:", "name")

local result = template.render("Hello, {{ name }}!", ctx)
log.info(result)
