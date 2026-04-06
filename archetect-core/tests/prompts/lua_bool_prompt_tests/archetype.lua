local ctx = Context.new()

if archetype.switch("test_lua_bool_prompt") then
    ctx:confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end

if archetype.switch("test_lua_bool_prompt_with_default") then
    ctx:confirm("Verbose:", "verbose", {
        default = true,
        help = "Enable verbose output",
    })
    log.info(tostring(ctx:get("verbose")))
end

if archetype.switch("test_lua_bool_prompt_non_optional") then
    ctx:confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end

if archetype.switch("test_lua_bool_prompt_with_answer") then
    ctx:confirm("Enable logging:", "enable_logging")
    log.info(tostring(ctx:get("enable_logging")))
end
