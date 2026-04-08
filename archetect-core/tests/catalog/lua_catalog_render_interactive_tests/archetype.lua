local context = Context.new()

-- Render "services/rest" which has no pre-answers,
-- so the child archetype will prompt interactively
catalog.render("services/rest", context)

log.info("interactive catalog render completed")
