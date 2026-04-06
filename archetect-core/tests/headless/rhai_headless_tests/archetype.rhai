let context = #{};

if switch_enabled("test_headless_with_defaults") {
    context.name = prompt("Name:", #{
        defaults_with: "DefaultName",
    });
    context.port = prompt("Port:", #{
        type: Int,
        defaults_with: 8080,
    });
    context.enabled = prompt("Enabled:", #{
        type: Bool,
        defaults_with: true,
    });
    debug(context);
}

if switch_enabled("test_headless_without_defaults") {
    context.name = prompt("Name:");
    debug(context);
}
