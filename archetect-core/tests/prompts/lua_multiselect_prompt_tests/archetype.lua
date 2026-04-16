local ctx = Context.new()

if archetype.switches.is_enabled("test_lua_multiselect_prompt") then
    ctx:prompt_multiselect("Languages:", "languages", {"Rust", "Java", "Go"})
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end

if archetype.switches.is_enabled("test_lua_multiselect_prompt_with_options") then
    ctx:prompt_multiselect("Languages:", "languages", {"Rust", "Java", "Go"}, {
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

if archetype.switches.is_enabled("test_lua_multiselect_prompt_non_optional") then
    ctx:prompt_multiselect("Languages:", "languages", {"Rust", "Java", "Go"})
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end

if archetype.switches.is_enabled("test_lua_multiselect_prompt_with_default") then
    ctx:prompt_multiselect("Languages:", "languages", {"Rust", "Java", "Go"}, {
        default = {"Rust", "Go"},
    })
    local langs = ctx:get("languages")
    if langs then
        for i, v in ipairs(langs) do
            log.info(v)
        end
    end
end
