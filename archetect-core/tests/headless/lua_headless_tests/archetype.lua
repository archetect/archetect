local ctx = Context.new()

if archetype.switches.is_enabled("test_headless_with_defaults") then
    ctx:prompt_text("Name:", "name", { default = "DefaultName" })
    ctx:prompt_int("Port:", "port", { default = 8080 })
    ctx:prompt_confirm("Enabled:", "enabled", { default = true })
    log.info(tostring(ctx:get("name")))
    log.info(tostring(ctx:get("port")))
    log.info(tostring(ctx:get("enabled")))
end

if archetype.switches.is_enabled("test_headless_without_defaults") then
    ctx:prompt_text("Name:", "name")
end
