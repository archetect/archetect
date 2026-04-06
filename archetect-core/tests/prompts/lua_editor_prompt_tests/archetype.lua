local ctx = Context.new()

if archetype.switch("test_lua_editor_prompt") then
    ctx:editor("Description:", "description")
    log.info(tostring(ctx:get("description")))
end

if archetype.switch("test_lua_editor_prompt_with_default") then
    ctx:editor("Description:", "description", {
        default = "Default description",
        help = "Enter a description",
    })
    log.info(tostring(ctx:get("description")))
end
