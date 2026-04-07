local ctx = Context.new()

ctx:prompt_text("Name:", "name")

local result = template.render("Hello, {{ name }}!", ctx)
log.info(result)
