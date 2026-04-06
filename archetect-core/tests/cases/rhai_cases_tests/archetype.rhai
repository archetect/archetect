let context = #{};

if switch_enabled("test_single_case_style") {
    context += set("project_name", "My Cool Project", #{
        cased_as: SnakeCase,
    });
    debug(context);
}
