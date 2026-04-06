let context = #{};

let alternate_answers = #{
    enable_logging: true,
};

if switch_enabled("test_scalar_bool_prompt") {
    context.enabled = prompt("Enabled:", #{
        type: Bool,
    });
    debug(context);
}

if switch_enabled("test_scalar_bool_prompt_with_default") {
    context.verbose = prompt("Verbose:", #{
        type: Bool,
        defaults_with: true,
        help: "Enable verbose output",
        placeholder: "true/false",
    });
    debug(context);
}

if switch_enabled("test_scalar_bool_prompt_non_optional") {
    context.enabled = prompt("Enabled:", #{
        type: Bool,
    });
    debug(context);
}

if switch_enabled("test_scalar_bool_prompt_unexpected") {
    context.enabled = prompt("Enabled:", #{
        type: Bool,
    });
    debug(context);
}

if switch_enabled("test_scalar_bool_prompt_with_answer") {
    context.enable_logging = prompt("Enable Logging:", #{
        type: Bool,
        answer_key: "enable_logging",
        answer_source: alternate_answers,
    });
    debug(context);
}
