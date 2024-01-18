let context = #{};

if switch_enabled("test_simple") {
    context.services = prompt("Services:", #{
        type: List,
    });
    print(as_yaml(context));
}

if switch_enabled("test_map") {
    context += prompt("Services:", "services", #{
        type: List,
    });
    print(as_yaml(context));
}

if switch_enabled("test_simple_cased") {
    context.services = prompt("Services:", #{
        type: List,
        cased_as: KebabCase,
    });
    print(as_yaml(context));
}

if switch_enabled("test_map_cased") {
    context += prompt("Services:", "services", #{
        type: List,
        cased_as: [
            CasedIdentityCasedValue([KebabCase]),
        ],
    });
    print(as_yaml(context));
}

if switch_enabled("test_map_cased_as_with_single_strategy") {
    context += prompt("Services:", "services", #{
        type: List,
        cased_as: CasedIdentityCasedValue([KebabCase]),
    });
    print(as_yaml(context));
}

if switch_enabled("test_simple_cased_as_with_single_style") {
    context.services = prompt("Services:", #{
        type: List,
        cased_as: KebabCase,
    });
    print(as_yaml(context));
}

if switch_enabled("test_map_cased_as_with_string_strategy") {
    context += prompt("Services:", "services", #{
        type: List,
        cased_as: [
            "CamelCase",
        ],
    });
    print(as_yaml(context));
}