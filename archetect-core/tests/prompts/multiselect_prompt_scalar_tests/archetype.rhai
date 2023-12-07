let context = #{};

if switch_enabled("test_scalar_defaults") {
    context.languages = prompt("Languages:", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
    });
    debug(context);
}

if switch_enabled("test_map_defaults") {
    context += prompt("Languages:", "languages", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
    });
    debug(context);
}

if switch_enabled("test_scalar_cased_as") {
    context.languages = prompt("Languages:", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
        cased_as: SnakeCase,
    });
    debug(context);
}

if switch_enabled("test_scalar_headless_defaults_headless") {
    context.languages = prompt("Languages:", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
        defaults_with: ["Rust", "JavaScript"],
    });
    debug(context);
}

if switch_enabled("test_scalar_headless_defaults_cased_as_headless") {
    context.languages = prompt("Languages:", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
        cased_as: CamelCase,
        defaults_with: ["Rust", "JavaScript"],
    });
    debug(context);
}

if switch_enabled("test_scalar_with_defaults") {
    context.languages = prompt("Languages:", #{
        type: MultiSelect([
            "Rust",
            "Java",
            "JavaScript",
        ]),
        defaults_with: ["Rust", "Java"],
    });
    debug(context);
}


