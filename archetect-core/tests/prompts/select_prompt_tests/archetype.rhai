let context = #{};

if switch_enabled("test_select_basic") {
    context.language = prompt("Language:", #{
        type: Select([
            "Rust",
            "Java",
            "Go",
        ]),
    });
    debug(context);
}

if switch_enabled("test_select_with_options") {
    context.language = prompt("Language:", #{
        type: Select([
            "Rust",
            "Java",
            "Go",
        ]),
        defaults_with: "Rust",
        help: "Choose your primary language",
        page_size: 5,
    });
    debug(context);
}

if switch_enabled("test_select_non_optional") {
    context.language = prompt("Language:", #{
        type: Select([
            "Rust",
            "Java",
            "Go",
        ]),
    });
    debug(context);
}

if switch_enabled("test_select_with_answer") {
    context.language = prompt("Language:", #{
        type: Select([
            "Rust",
            "Java",
            "Go",
        ]),
        answer_key: "language",
    });
    debug(context);
}
