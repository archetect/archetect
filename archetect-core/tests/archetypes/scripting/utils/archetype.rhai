log(Trace, "Trace Level");
log(Debug, "Debug Level");
log(Info, "Info Level");
log(Warn, "Warn Level");
log(Error, "Error Level");

display("Display Message");
display();

print("Print Message");
print();

debug("Debug Message");
debug(["Debug", "Message"]);

print(uuid());

print(switch_enabled("build"));
print(switch_enabled("test"));

Archetype("child1").render(#{});

Archetype("child2").render(#{}, #{
    switches: SWITCHES + ["print"],
});

