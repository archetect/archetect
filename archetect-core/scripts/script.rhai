

print(answers);
print(context);

context["first_name"] = "Jimmie";
context["last-name"] = "Fulton";

context.services = #{};

context.services += prompt("Project Prefix:", "prefix", #{
    help: "The first part of your service name",
    placeholder: "Cart",
});

context.services += prompt("Project Suffix:", "suffix", #{
    defaultValue: "Service",
    help: "The second part of your service name",
});

context.services["artifact-id"] = render("{{ prefix | train_case }}-{{suffix | train_case }}", context);
context.services["artifact-id"] = kebab_case(`${ context.prefix }-${ context.suffix}", context);
context.services["artifact-title"] = render("{{ prefix | title_case }} {{suffix | title_case }}", context);

log(INFO, render("Hello, {{ first_name }}!", context));

mount("contents", context);

mount("layouts_base", context, #{
    rules: [],
});

print(context);

let lastname = "Fulton";

print(context);

// let result = render("{{ services.service1 | pluralize | upper_case }}", context);
// print(result);

