local ctx = Context.new()

if archetype.switch("test_headless_with_defaults") then
    ctx:text("Name:", "name", { default = "DefaultName" })
    ctx:int("Port:", "port", { default = 8080 })
    ctx:confirm("Enabled:", "enabled", { default = true })
    log.info(tostring(ctx:get("name")))
    log.info(tostring(ctx:get("port")))
    log.info(tostring(ctx:get("enabled")))
end

if archetype.switch("test_headless_without_defaults") then
    ctx:text("Name:", "name")
end
