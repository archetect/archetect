local ctx = Context.new()

if archetype.switch("test_lua_int_prompt") then
    ctx:prompt_int("Port:", "port")
    log.info(tostring(ctx:get("port")))
end

if archetype.switch("test_lua_int_prompt_with_options") then
    ctx:prompt_int("Port:", "port", {
        default = 8080,
        min = 1024,
        max = 65535,
        help = "Enter a port number",
        placeholder = "8080",
    })
    log.info(tostring(ctx:get("port")))
end

if archetype.switch("test_lua_int_prompt_non_optional") then
    ctx:prompt_int("Port:", "port")
    log.info(tostring(ctx:get("port")))
end
