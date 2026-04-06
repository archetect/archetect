if switch_enabled("test_as_json") {
    let context = #{
        name: "Alice",
        age: 30,
    };
    let json = as_json(context);
    display(json);
}

if switch_enabled("test_as_yaml") {
    let context = #{
        name: "Alice",
        age: 30,
    };
    let yaml = as_yaml(context);
    display(yaml);
}

if switch_enabled("test_from_json") {
    let json = "{\"name\": \"Bob\", \"age\": 25}";
    let context = from_json(json);
    display(context.name);
    display(`${context.age}`);
}

if switch_enabled("test_from_yaml") {
    let yaml = "name: Charlie\nage: 35";
    let context = from_yaml(yaml);
    display(context.name);
    display(`${context.age}`);
}
