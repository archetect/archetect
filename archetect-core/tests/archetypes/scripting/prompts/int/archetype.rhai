let context = #{};

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

context.debug_port = prompt("Debug Port:", #{
    type: Int,
    answer_key: "debug_port",
});

debug(context);