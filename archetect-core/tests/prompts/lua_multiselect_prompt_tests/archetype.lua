local ctx = Context.new()

if archetype.switch("test_lua_multiselect_prompt") then
    ctx:prompt_multi_select("Languages:", "languages", {"Rust", "Java", "Go"})
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end

if archetype.switch("test_lua_multiselect_prompt_with_options") then
    ctx:prompt_multi_select("Languages:", "languages", {"Rust", "Java", "Go"}, {
        help = "Select your languages",
        min = 1,
        max = 2,
    })
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end

if archetype.switch("test_lua_multiselect_prompt_non_optional") then
    ctx:prompt_multi_select("Languages:", "languages", {"Rust", "Java", "Go"})
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end
