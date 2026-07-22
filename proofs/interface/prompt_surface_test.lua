-- Dynamic-interface phase 1, proven black-box: the prompt surface says everything
-- interface.yaml could say — and unlike interface.yaml, saying it makes it SO.
-- (docs/plans/dynamic-interface.md §2 — pattern, rich options, group/ui metadata.)

local workspace = require("prova.workspace")

local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

-- A fixture archetype exercising every phase-1 surface at once.
local arch = prova.fixture("phase1-archetype", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	ws:write("archetype/archetype.yaml", [[
description: "Phase 1 prompt-surface fixture"
requires:
  archetect: "3.0.0"
]])
	ws:write("archetype/archetype.lua", [[
local context = Context.new()

context:prompt_text("Service Name:", "service_name", {
  pattern = "^[a-z][a-z0-9-]*$",
  help = "Lowercase kebab identifier",
  group = "Identity",
  ui = { widget = "text", advanced = false },
})

context:prompt_select("Persistence:", "persistence", {
  { value = "pg", label = "PostgreSQL", help = "Production-grade" },
  "none",
}, { default = "pg", group = "Storage" })

context:prompt_multiselect("Features:", "features", {
  { value = "m", label = "Metrics" },
  { value = "t", label = "Tracing" },
  "health",
}, { default = { "m" } })

directory.render("content", context)
]])
	ws:write("archetype/content/out.toml", [[
name = "{{ service_name }}"
persistence = "{{ persistence }}"
features = [{% for _, f in ipairs(features) do %}"{{ f }}", {% end %}]
]])
	return ws
end)

local function render(t, ws, dest, args)
	local cmd = { t:use(bin), "render", ws:file("archetype"), "--destination", ws:file(dest), "--headless" }
	for _, a in ipairs(args) do
		table.insert(cmd, a)
	end
	return shell.run(cmd)
end

-- ── pattern: validation that actually validates ────────────────────

prova.test("a pattern-conforming answer renders", function(t)
	local ws = t:use(arch)
	local out = render(t, ws, "ok", { "-a", "service_name=my-service", "-D" })
	t:expect(out.code, "exit"):equals(0)
	t:expect(ws:read("ok/out.toml")):contains('name = "my-service"')
end)

prova.test("a pattern violation is an error naming the key and the pattern", function(t)
	local ws = t:use(arch)
	local out = render(t, ws, "bad", { "-a", "service_name=Bad Name!", "-D" })
	t:expect(out.code, "exit"):never():equals(0)
	t:expect(out.stderr, "names the key"):contains("service_name")
	t:expect(out.stderr, "names the pattern"):contains("^[a-z][a-z0-9-]*$")
end)

-- ── rich options: {value, label, help} beside bare strings ─────────

prova.test("rich select options default and store by VALUE, not label", function(t)
	local ws = t:use(arch)
	local out = render(t, ws, "defaults", { "-a", "service_name=svc", "-D" })
	t:expect(out.code, "exit"):equals(0)
	t:expect(ws:read("defaults/out.toml")):contains('persistence = "pg"')
end)

prova.test("rich select options answer by value; mixed string form still works", function(t)
	local ws = t:use(arch)
	local out = render(t, ws, "byvalue", { "-a", "service_name=svc", "-a", "persistence=none", "-D" })
	t:expect(out.code, "exit"):equals(0)
	t:expect(ws:read("byvalue/out.toml")):contains('persistence = "none"')
end)

prova.test("rich multiselect options answer and default by value", function(t)
	local ws = t:use(arch)
	local d = render(t, ws, "msdefault", { "-a", "service_name=svc", "-D" })
	t:expect(d.code, "exit"):equals(0)
	t:expect(ws:read("msdefault/out.toml")):contains('features = ["m", ]')
	local a = render(t, ws, "msanswer", { "-a", "service_name=svc", "-a", "features=[t,health]", "-D" })
	t:expect(a.code, "exit"):equals(0)
	t:expect(ws:read("msanswer/out.toml")):contains('features = ["t", "health", ]')
end)

-- ── the envelope carries the whole declaration (MCP, pre-scripted stdio) ──

-- The MCP server dispatches tool calls concurrently, so a blind stdio
-- pipe cannot sequence respond-after-render. Each session below makes
-- exactly ONE in-flight call: render with just enough answers that the
-- envelope under test is the first prompt returned.
local function mcp_first_prompt(t, ws, tag, answers_json)
	local reqs = table.concat({
		'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"proof","version":"0"}}}',
		'{"jsonrpc":"2.0","method":"notifications/initialized"}',
		'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"render","arguments":{"source":"'
			.. ws:file("archetype") .. '","destination":"' .. ws:file("mcp-out-" .. tag) .. '"'
			.. (answers_json and (',"answers":' .. answers_json) or "") .. '}}}',
		"",
	}, "\n")
	local reqfile = ws:write("mcp-requests-" .. tag .. ".jsonl", reqs)
	local out = shell.run({ "sh", "-c", t:use(bin) .. " mcp < " .. reqfile }, { timeout = "60s" })
	for line in string.gmatch(out.stdout, "[^\n]+") do
		local ok, msg = pcall(prova.parse.json, line)
		if ok and type(msg) == "table" and msg.id == 2 then
			return prova.parse.json(msg.result.content[1].text).prompt
		end
	end
	return nil
end

prova.test("MCP text envelope carries pattern, group, and ui", function(t)
	local ws = t:use(arch)
	local p = mcp_first_prompt(t, ws, "text", nil)
	t:expect(p ~= nil, "render returned a prompt"):equals(true)
	t:expect(p.key):equals("service_name")
	t:expect(p.pattern, "pattern rides the envelope"):equals("^[a-z][a-z0-9-]*$")
	t:expect(p.group, "group rides the envelope"):equals("Identity")
	t:expect(p.ui.widget, "ui passes through opaquely"):equals("text")
end)

prova.test("MCP select envelope carries option values, labels, and help", function(t)
	local ws = t:use(arch)
	local p = mcp_first_prompt(t, ws, "select", '{"service_name":"svc"}')
	t:expect(p ~= nil, "render returned a prompt"):equals(true)
	t:expect(p.key):equals("persistence")
	t:expect(p.group, "group rides the envelope"):equals("Storage")
	t:expect(p.options[1].value, "options carry values"):equals("pg")
	t:expect(p.options[1].label, "options carry labels"):equals("PostgreSQL")
	t:expect(p.options[1].help, "options carry help"):equals("Production-grade")
	t:expect(p.options[2].value, "bare strings normalize"):equals("none")
	t:expect(p.options[2].label, "bare string label == value"):equals("none")
end)

-- ── the binary teaches the new surface (autodidact ratchet) ────────

prova.test("introspect teaches pattern, group, ui, and rich options", function(t)
	local b = t:use(bin)
	local text_opts = shell.run({ b, "introspect", "TextPromptOpts" }, { check = true }).stdout
	t:expect(text_opts):contains("pattern")
	t:expect(text_opts):contains("group")
	t:expect(text_opts):contains("ui")
	local select_sig = shell.run({ b, "introspect", "prompt_select" }, { check = true }).stdout
	t:expect(select_sig, "options accept rich tables"):contains("SelectOption")
end)

prova.test("learn prompts documents the phase-1 surface", function(t)
	local out = shell.run({ t:use(bin), "learn", "prompts" }, { check = true })
	t:expect(out.stdout):contains("pattern")
	t:expect(out.stdout, "rich option form"):contains("value")
end)
