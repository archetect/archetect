local ctx = Context.new()

if archetype.switch("test_lua_list_prompt") then
    ctx:list("Dependencies:", "dependencies")
    local deps = ctx:get("dependencies")
    if deps then
        for i, v in ipairs(deps) do
            log.info(v)
        end
    end
end

if archetype.switch("test_lua_list_prompt_with_options") then
    ctx:list("Dependencies:", "dependencies", {
        help = "Enter dependencies one at a time",
        min = 1,
        max = 5,
    })
    local deps = ctx:get("dependencies")
    if deps then
        for i, v in ipairs(deps) do
            log.info(v)
        end
    end
end
