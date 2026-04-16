local ctx = Context.new()

if archetype.switches.is_enabled("test_lua_cases_programming") then
    ctx:set("project_name", "My Cool Project", {
        cases = Cases.programming()
    })

    -- The original key is stored, then cases expand additional keys.
    -- Snake of "project_name" is "project_name" which overwrites original with cased value.
    log.info(tostring(ctx:get("project_name")))       -- snake overwrites: "my_cool_project"
    log.info(tostring(ctx:get("ProjectName")))         -- pascal value
    log.info(tostring(ctx:get("projectName")))         -- camel value
    log.info(tostring(ctx:get("project-name")))        -- kebab value
    log.info(tostring(ctx:get("Project-Name")))        -- train value
    log.info(tostring(ctx:get("PROJECT_NAME")))        -- constant value
end

if archetype.switches.is_enabled("test_lua_cases_enum_set") then
    -- Cases.set() with Case enum constants
    ctx:set("app_name", "My App", {
        cases = Cases.set(Case.Snake, Case.Kebab, Case.Constant)
    })
    log.info(tostring(ctx:get("app_name")))       -- snake: "my_app"
    log.info(tostring(ctx:get("app-name")))        -- kebab: "my-app"
    log.info(tostring(ctx:get("APP_NAME")))        -- constant: "MY_APP"
end

if archetype.switches.is_enabled("test_lua_cases_enum_fixed") then
    -- Cases.fixed() with Case enum constant
    ctx:set("name", "Hello World", {
        cases = Cases.fixed("display_name", Case.Title)
    })
    log.info(tostring(ctx:get("name")))            -- original: "Hello World"
    log.info(tostring(ctx:get("display_name")))    -- fixed key, title-cased value
end

if archetype.switches.is_enabled("test_lua_cases_input") then
    -- Cases.input("key") preserves the untransformed input under an explicit key.
    -- Useful when Cases.programming() auto-cases the primary key's value but
    -- you still need access to the literal user input.
    ctx:set("project_name", "My Cool Project", {
        cases = { Cases.programming(), Cases.input("project_name_raw") }
    })
    log.info(tostring(ctx:get("project_name")))       -- snake-cased: "my_cool_project"
    log.info(tostring(ctx:get("project_name_raw")))   -- untransformed: "My Cool Project"
    log.info(tostring(ctx:get("ProjectName")))         -- pascal still works: "MyCoolProject"
end

if archetype.switches.is_enabled("test_lua_cases_input_with_prompt") then
    -- Same pattern but via prompt_text (headless + answer)
    ctx:prompt_text("Name:", "widget_name", {
        cases = { Cases.programming(), Cases.input("widget_name_raw") }
    })
    log.info(tostring(ctx:get("widget_name")))         -- snake-cased
    log.info(tostring(ctx:get("widget_name_raw")))     -- untransformed
    log.info(tostring(ctx:get("widget-name")))          -- kebab variant
end
