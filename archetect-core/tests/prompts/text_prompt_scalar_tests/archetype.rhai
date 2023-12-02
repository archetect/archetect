let context = #{};

let alternate_answers = #{
    summary: "Extended Summary",
};

context.service_prefix = prompt("Service Prefix:");

context.service_suffix = prompt("Service Suffix:", #{
    defaults_with: "Orchestrator",
    min: 2,
    max: 15,
    placeholder: "Service",
    help: "Enter a Service Suffix",
    optional: false,
    answer_key: "service_suffix",
});

// Get answer provided by RenderContext
context.description = prompt("Description:", #{
    optional: true,
    answer_key: "description",
});

// Get answer from within Scripting Context
context.summary = prompt("Summary", #{
    answer_key: "summary",
    answer_source: alternate_answers,
});

debug(context);