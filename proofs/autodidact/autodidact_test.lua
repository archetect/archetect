-- The autodidact surface, proven black-box: the shipped binary teaches its own drivers.
-- (docs/plans/autodidact.md — the acceptance bar for M0–M3, held by prova.)

-- The binary under proof: built once per file, from this workspace.
local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

local TOPICS = {
	"generation", "environment", "rendering", "authoring", "manifest", "prompts",
	"cases", "templates", "catalogs", "composition", "model", "sources", "mcp",
}

prova.test("learn lists every topic", function(t)
	local out = shell.run({ t:use(bin), "learn" }, { check = true })
	for _, topic in ipairs(TOPICS) do
		t:expect(out.stdout, "listing"):contains(topic)
	end
end)

prova.test("every agent-facing verb is taught by some topic or the skill", function(t)
	local b = t:use(bin)
	local corpus = shell.run({ b, "skill" }, { check = true }).stdout
	for _, topic in ipairs(TOPICS) do
		corpus = corpus .. shell.run({ b, "learn", topic }, { check = true }).stdout
	end
	-- The ratchet: every verb an agent reaches for must appear in the taught corpus.
	-- (`global`, `server`, `connect`, `completions` are not yet taught — add them to a topic,
	-- then move them into this list.)
	local taught = {
		"render", " ls", "search", "config", "cache", "check", "ide setup",
		"learn", "introspect", "eval", "skill", "mcp", "system layout",
	}
	t:expect_all(function()
		for _, verb in ipairs(taught) do
			t:expect(corpus, "verb " .. verb):contains(verb)
		end
	end)
end)

prova.test("a topic computes THIS environment, not prose about one", function(t)
	local out = shell.run({ t:use(bin), "learn", "environment" }, { check = true })
	t:expect(out.stdout):contains("## Here, right now")
	t:expect(out.stdout, "no leaked slot"):never():contains("[[slot:")
end)

prova.test("aliases resolve (atl → templates)", function(t)
	local out = shell.run({ t:use(bin), "learn", "atl" }, { check = true })
	t:expect(out.stdout):contains("# templates")
end)

prova.test("an unknown topic errors and carries the listing", function(t)
	local out = shell.run({ t:use(bin), "learn", "definitely-not-a-topic" })
	t:expect(out.code, "exit"):never():equals(0)
	t:expect(out.stderr):contains("unknown topic")
	t:expect(out.stderr, "the listing rides the error"):contains("generation")
end)

prova.test("introspect answers the shapes that once required reading source", function(t)
	local out = shell.run({ t:use(bin), "introspect", "prompt_select" }, { check = true })
	t:expect(out.stdout):contains("Context:prompt_select")
	t:expect(out.stdout, "signature"):contains("(message: string, key: string")
end)

prova.test("eval probes live behavior: filters, cases, inflections", function(t)
	local out = shell.run({
		t:use(bin), "eval",
		'local c = Context.new() c:set("x", "customer order") '
			.. 'return template.render("{{ x | train_case }}+{{ x | pluralize }}", c)',
	}, { check = true })
	t:expect(out.stdout):contains("Customer-Order+customer orders")
end)

prova.test("eval is headless: a prompting probe errors instead of hanging", function(t)
	local out = shell.run({
		t:use(bin), "eval",
		'local c = Context.new() c:prompt_text("Name:", "name") return c:get("name")',
	}, { timeout = "30s" })
	t:expect(out.code, "exit"):never():equals(0)
end)

prova.test("mcp serves identity and knowledge over stdio", function(t)
	local b = t:use(bin)
	local requests = table.concat({
		'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"prova","version":"0"}}}',
		'{"jsonrpc":"2.0","method":"notifications/initialized"}',
		'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"learn","arguments":{}}}',
		'{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"archetect://skill"}}',
	}, "\n")
	local out = shell.run({ "sh", "-c", "printf '%s\\n' '" .. requests:gsub("\n", "' '") .. "' | " .. b .. " mcp" },
		{ timeout = "60s", check = true })
	t:expect(out.stdout, "instructions"):contains("render, don't hand-write")
	t:expect(out.stdout, "learn tool listing"):contains("progressive disclosure")
	t:expect(out.stdout, "skill resource"):contains("archetect learn")
end)
