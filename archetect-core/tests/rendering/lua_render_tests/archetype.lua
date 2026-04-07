local ctx = Context.new()

ctx:prompt_text("Project Name:", "project_name")

directory.render("default", ctx)
