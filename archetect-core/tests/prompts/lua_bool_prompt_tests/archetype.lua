local ctx = Context.new()

if archetect.switches.is_enabled("test_lua_bool_prompt") then
    ctx:prompt_confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end

if archetect.switches.is_enabled("test_lua_bool_prompt_with_default") then
    ctx:prompt_confirm("Verbose:", "verbose", {
        default = true,
        help = "Enable verbose output",
    })
    log.info(tostring(ctx:get("verbose")))
end

if archetect.switches.is_enabled("test_lua_bool_prompt_non_optional") then
    ctx:prompt_confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end

if archetect.switches.is_enabled("test_lua_bool_prompt_with_answer") then
    ctx:prompt_confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end
