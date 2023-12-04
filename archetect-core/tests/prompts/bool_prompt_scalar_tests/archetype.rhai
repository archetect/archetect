let context = #{};

let alternate_answers = #{
    rest_port: 8060,
};

context.service_port = prompt("Service Port:", #{
    type: Int,
});

context.management_port = prompt("Management Port:", #{
    defaults_with: context.service_port + 1,
    type: Int,
    min: 1024,
    max: 65535,
    placeholder: "Management Port Number",
    help: "Enter an integer between 1024 and 65535",
    optional: true,
});

// Get answer provided by RenderContext
context.debug_port = prompt("Debug Port:", #{
    type: Int,
    answer_key: "debug_port",
});

// Get answer from within Scripting Context
context.rest_port = prompt("Rest Port:", #{
    type: Int,
    answer_key: "rest_port",
    answer_source: alternate_answers,
});

debug(context);