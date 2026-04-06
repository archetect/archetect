local ctx = Context.new()

if archetype.switch("test_lua_text_prompt") then
    ctx:text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end

if archetype.switch("test_lua_text_prompt_with_options") then
    ctx:text("Service Name:", "service_name", {
        default = "MyService",
        min = 2,
        max = 20,
        help = "Enter a service name",
        placeholder = "ServiceName",
    })
    log.info(tostring(ctx:get("service_name")))
end

if archetype.switch("test_lua_text_prompt_non_optional") then
    ctx:text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end

if archetype.switch("test_lua_text_prompt_with_answer") then
    ctx:text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end
