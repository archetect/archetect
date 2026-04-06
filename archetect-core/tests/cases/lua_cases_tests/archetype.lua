local ctx = Context.new()

if archetype.switch("test_lua_cases_programming") then
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

if archetype.switch("test_lua_cases_enum_set") then
    -- Cases.set() with Case enum constants
    ctx:set("app_name", "My App", {
        cases = Cases.set(Case.Snake, Case.Kebab, Case.Constant)
    })
    log.info(tostring(ctx:get("app_name")))       -- snake: "my_app"
    log.info(tostring(ctx:get("app-name")))        -- kebab: "my-app"
    log.info(tostring(ctx:get("APP_NAME")))        -- constant: "MY_APP"
end

if archetype.switch("test_lua_cases_enum_fixed") then
    -- Cases.fixed() with Case enum constant
    ctx:set("name", "Hello World", {
        cases = Cases.fixed("display_name", Case.Title)
    })
    log.info(tostring(ctx:get("name")))            -- original: "Hello World"
    log.info(tostring(ctx:get("display_name")))    -- fixed key, title-cased value
end
