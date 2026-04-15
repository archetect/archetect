local ctx = Context.new()

if archetect.switches.is_enabled("test_lua_editor_prompt") then
    ctx:prompt_editor("Description:", "description")
    log.info(tostring(ctx:get("description")))
end

if archetect.switches.is_enabled("test_lua_editor_prompt_with_default") then
    ctx:prompt_editor("Description:", "description", {
        default = "Default description",
        help = "Enter a description",
    })
    log.info(tostring(ctx:get("description")))
end
