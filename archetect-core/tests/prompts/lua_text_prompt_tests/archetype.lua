local ctx = Context.new()

if archetect.switches.is_enabled("test_lua_text_prompt") then
    ctx:prompt_text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end

if archetect.switches.is_enabled("test_lua_text_prompt_with_options") then
    ctx:prompt_text("Service Name:", "service_name", {
        default = "MyService",
        min = 2,
        max = 20,
        help = "Enter a service name",
        placeholder = "ServiceName",
    })
    log.info(tostring(ctx:get("service_name")))
end

if archetect.switches.is_enabled("test_lua_text_prompt_non_optional") then
    ctx:prompt_text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end

if archetect.switches.is_enabled("test_lua_text_prompt_with_answer") then
    ctx:prompt_text("Service Name:", "service_name")
    log.info(tostring(ctx:get("service_name")))
end
