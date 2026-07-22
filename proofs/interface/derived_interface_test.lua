-- Dynamic-interface phases 2–4, proven black-box: the interface is something
-- you ASK the archetype (probe → transcript), not something its author claims.
-- (docs/plans/dynamic-interface.md §3–§5.)

local workspace = require("prova.workspace")

local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

-- One workspace, several fixture archetypes.
local ws_fixture = prova.fixture("interface-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- flat: every prompt defaulted; queries two switches; writes one file.
	ws:write("flat/archetype.yaml", 'description: "Flat"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("flat/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders", pattern = "^[a-z][a-z0-9-]*$", group = "Identity" })
context:prompt_int("Port:", "port", { default = 8080, min = 1024, max = 65535 })
context:prompt_confirm("Telemetry:", "telemetry", { default = true })
if archetype.switches.is_enabled("ci") then context:set("ci", true) end
if archetype.switches.is_enabled("docker") then context:set("docker", true) end
directory.render("content", context)
]])
	ws:write("flat/content/out.toml", 'name = "{{ service_name }}"\nport = {{ port }}\n')

	-- conditional: the db_name prompt hides behind persistence = "pg".
	ws:write("conditional/archetype.yaml", 'description: "Conditional"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("conditional/archetype.lua", [[
local context = Context.new()
context:prompt_select("Persistence:", "persistence", { "none", "pg" }, { default = "none" })
if context:get("persistence") == "pg" then
  context:prompt_text("Database Name:", "db_name", { default = "app" })
end
]])

	-- looping: an unbounded prompt loop the budget must stop.
	ws:write("looping/archetype.yaml", 'description: "Looping"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("looping/archetype.lua", [[
local context = Context.new()
local i = 0
while true do
  i = i + 1
  context:prompt_text("Item " .. i .. ":", "item_" .. i, { optional = true })
end
]])

	-- composed: parent renders a child via its catalog (relative source).
	ws:write("composed/archetype.yaml", table.concat({
		'description: "Composed"',
		'requires:',
		'  archetect: "3.0.0"',
		'catalog:',
		'  child:',
		'    source: "./child"',
		'',
	}, "\n"))
	ws:write("composed/archetype.lua", [[
local context = Context.new()
context:prompt_text("Parent Name:", "parent_name", { default = "parent" })
catalog.render("child", context)
]])
	ws:write("composed/child/archetype.yaml", 'description: "Child"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("composed/child/archetype.lua", [[
local context = Context.new()
context:prompt_text("Child Name:", "child_name", { default = "child" })
]])

	-- declared-inline: still carries an `interface:` block (removed feature).
	ws:write("declared-inline/archetype.yaml", table.concat({
		'description: "Declared inline"',
		'requires:',
		'  archetect: "3.0.0"',
		'interface:',
		'  prompts:',
		'    - key: phantom_key',
		'      type: text',
		'      label: "Never asked"',
		'',
	}, "\n"))
	ws:write("declared-inline/archetype.lua", [[
local context = Context.new()
context:prompt_text("Real Key:", "real_key", { default = "x" })
]])

	-- declared-sibling: still ships an interface.yaml file (removed feature).
	ws:write("declared-sibling/archetype.yaml", 'description: "Declared sibling"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("declared-sibling/interface.yaml", 'prompts:\n  - key: x\n    type: text\n    label: "X"\n')
	ws:write("declared-sibling/archetype.lua", 'local context = Context.new()\n')

	return ws
end)

local function interface_json(t, ws, rel, flags)
	local cmd = { t:use(bin), "interface", ws:file(rel), "--json" }
	for _, f in ipairs(flags or {}) do
		table.insert(cmd, f)
	end
	local out = shell.run(cmd, { check = true })
	return prova.parse.json(out.stdout)
end

-- ── the probe: transcript, switches, no side effects ───────────────

prova.test("interface derives every prompt with full metadata", function(t)
	local ws = t:use(ws_fixture)
	local result = interface_json(t, ws, "flat")
	t:expect(#result.prompts):equals(3)
	t:expect(result.prompts[1].key):equals("service_name")
	t:expect(result.prompts[1].pattern):equals("^[a-z][a-z0-9-]*$")
	t:expect(result.prompts[1].group):equals("Identity")
	t:expect(result.prompts[2].key):equals("port")
	t:expect(result.prompts[2].default):equals(8080)
	t:expect(result.prompts[3].type):equals("bool")
	t:expect(result.completed):equals(true)
end)

prova.test("switch queries are recorded without any switch being set", function(t)
	local ws = t:use(ws_fixture)
	local result = interface_json(t, ws, "flat")
	t:expect(result.switches[1]):equals("ci")
	t:expect(result.switches[2]):equals("docker")
end)

prova.test("the probe writes nothing", function(t)
	local ws = t:use(ws_fixture)
	interface_json(t, ws, "flat")
	t:expect(ws:exists("flat-out"), "no output dir appears"):equals(false)
	local find = shell.run({ "sh", "-c", "ls " .. ws.path .. " | grep -c out || true" })
	t:expect(find.stdout):contains("0")
end)

prova.test("composition descends: child prompts join the transcript", function(t)
	local ws = t:use(ws_fixture)
	local result = interface_json(t, ws, "composed")
	local keys = {}
	for _, p in ipairs(result.prompts) do
		keys[#keys + 1] = p.key
	end
	t:expect(table.concat(keys, ",")):contains("parent_name")
	t:expect(table.concat(keys, ","), "child prompt recorded"):contains("child_name")
end)

prova.test("an unbounded prompt loop trips the budget instead of hanging", function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({ t:use(bin), "interface", ws:file("looping"), "--json" }, { timeout = "60s" })
	local result = prova.parse.json(out.stdout)
	t:expect(result.budget_hit):equals(true)
	t:expect(result.coverage):equals("partial")
	t:expect(result.mode):equals("interactive")
end)

-- ── exploration: the prompt graph ──────────────────────────────────

prova.test("default-path coverage marks itself honestly", function(t)
	local ws = t:use(ws_fixture)
	local result = interface_json(t, ws, "conditional")
	t:expect(result.coverage):equals("default-path")
	t:expect(result.mode, "single path never proves batch"):equals("interactive")
	t:expect(#result.prompts, "db_name hidden on the default path"):equals(1)
end)

prova.test("exploration finds branch-hidden prompts and computes batch mode", function(t)
	local ws = t:use(ws_fixture)
	local result = interface_json(t, ws, "conditional", { "--explore" })
	t:expect(result.coverage):equals("complete")
	t:expect(result.mode, "fully mapped => batch is a computed fact"):equals("batch")
	local db
	for _, p in ipairs(result.prompts) do
		if p.key == "db_name" then db = p end
	end
	t:expect(db ~= nil, "db_name discovered"):equals(true)
	t:expect(db.appears_when[1].key):equals("persistence")
	t:expect(db.appears_when[1].equals):equals("pg")
end)

-- ── the headless-instructions artifact ─────────────────────────────

prova.test("answers-template round-trips to a zero-prompt render", function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run(
		{ t:use(bin), "interface", ws:file("flat"), "--answers-template" },
		{ check = true }
	)
	local template = ws:write("flat-answers.yaml", out.stdout)
	local render = shell.run({
		t:use(bin), "render", ws:file("flat"),
		"--destination", ws:file("flat-render"),
		"--headless", "-A", template,
	})
	t:expect(render.code, "template answers everything"):equals(0)
	t:expect(ws:read("flat-render/out.toml")):contains('name = "orders"')
end)

-- ── the declared interface is GONE: carrying one is a hard error ───

prova.test("an inline declared interface is a load error naming the migration", function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("declared-inline"),
		"--destination", ws:file("inline-render"), "--headless", "-D",
	})
	t:expect(out.code, "removed feature is an error"):never():equals(0)
	t:expect(out.stderr, "says what happened"):contains("no longer supported")
	t:expect(out.stderr, "points at the replacement"):contains("archetect interface")
end)

prova.test("a sibling interface.yaml is a load error naming the migration", function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("declared-sibling"),
		"--destination", ws:file("sibling-render"), "--headless", "-D",
	})
	t:expect(out.code, "removed feature is an error"):never():equals(0)
	t:expect(out.stderr):contains("interface.yaml")
	t:expect(out.stderr, "points at the replacement"):contains("archetect interface")
end)

-- ── MCP describe mirrors the CLI ───────────────────────────────────

prova.test("MCP describe returns the same derived interface as --json", function(t)
	local ws = t:use(ws_fixture)
	local reqs = table.concat({
		'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"proof","version":"0"}}}',
		'{"jsonrpc":"2.0","method":"notifications/initialized"}',
		'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"describe","arguments":{"source":"'
			.. ws:file("flat") .. '"}}}',
		"",
	}, "\n")
	local reqfile = ws:write("describe-requests.jsonl", reqs)
	local out = shell.run({ "sh", "-c", t:use(bin) .. " mcp < " .. reqfile }, { timeout = "60s" })
	local described
	for line in string.gmatch(out.stdout, "[^\n]+") do
		local ok, msg = pcall(prova.parse.json, line)
		if ok and type(msg) == "table" and msg.id == 2 then
			described = prova.parse.json(msg.result.content[1].text)
		end
	end
	t:expect(described ~= nil, "describe answered"):equals(true)
	t:expect(#described.prompts):equals(3)
	t:expect(described.prompts[1].key):equals("service_name")
	t:expect(described.switches[1]):equals("ci")
end)

