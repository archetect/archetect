local ctx = Context.new()

if archetype.switch("test_lua_select_prompt") then
    ctx:prompt_select("Language:", "language", {"Rust", "Java", "Go"})
    log.info(tostring(ctx:get("language")))
end

if archetype.switch("test_lua_select_prompt_with_options") then
    ctx:prompt_select("Language:", "language", {"Rust", "Java", "Go"}, {
        default = "Rust",
        help = "Choose your primary language",
    })
    log.info(tostring(ctx:get("language")))
end

if archetype.switch("test_lua_select_prompt_non_optional") then
    ctx:prompt_select("Language:", "language", {"Rust", "Java", "Go"})
    log.info(tostring(ctx:get("language")))
end

if archetype.switch("test_lua_select_prompt_with_answer") then
    ctx:prompt_select("Language:", "language", {"Rust", "Java", "Go"})
    log.info(tostring(ctx:get("language")))
end
