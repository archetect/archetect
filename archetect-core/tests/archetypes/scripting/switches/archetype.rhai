debug(SWITCHES);

print(switch_enabled("build"));
print(switch_enabled("test"));

Archetype("child1").render(#{});

Archetype("child2").render(#{}, #{
    switches: SWITCHES + ["print"],
});

