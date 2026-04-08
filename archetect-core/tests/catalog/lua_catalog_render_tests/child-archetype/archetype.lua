local context = Context.new()

context:prompt_text("Service Name:", "service_name")

directory.render("default", context)
