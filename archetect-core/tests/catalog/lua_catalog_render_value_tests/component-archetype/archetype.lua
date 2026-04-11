-- A mock component archetype: sets a key on its context and returns it.
-- The parent invokes this via catalog.render and gets back the modified
-- context as a fresh value (Lua's natural assignment semantics).
local context = Context.new()
context:set("set-by-component", "yes")
return context
