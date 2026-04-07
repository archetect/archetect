local ctx = Context.new()

ctx:prompt_text("Project Name:", "project_name")

archetype.render("default", ctx)
