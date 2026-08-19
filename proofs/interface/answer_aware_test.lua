-- Deriving an interface for what is STILL unknown, not for everything askable.
--
-- A form generator's real question is "what must I ask this user?", and that
-- depends on what the caller already knows. Ybor Studio supplies the GitHub
-- settings from its own configuration and wants them gone from the form; a
-- wizard that has collected page one wants page two's conditionals resolved
-- against those answers rather than shipped as predicates for every client to
-- reinvent.
--
-- The render session already behaves this way — a supplied answer never
-- surfaces — so this is the derived interface catching up to the thing it
-- describes. (docs/plans/dynamic-interface.md#describe-accepts-answers)

local workspace = require("prova.workspace")

local bin = require("subject").bin

local ws_fixture = prova.fixture("answer-aware-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- One archetype covering both halves: a prompt the caller can supply, and a
	-- branch whose continuation only exists once an answer selects it.
	ws:write("service/archetype.yaml", 'description: "Service"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("service/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
context:prompt_text("GitHub Visibility:", "github_visibility", { default = "private" })
context:prompt_select("Messaging:", "messaging", { "none", "kafka" }, { default = "none" })
if context:get("messaging") == "kafka" then
  context:prompt_text("Kafka Topic:", "kafka_topic", { default = "events" })
end
if archetype.switches.is_enabled("ci") then
  context:prompt_text("CI Runner:", "ci_runner", { default = "ubuntu" })
end
]])

	return ws
end)

local function derive(t, ws, args)
	local cmd = { t:use(bin), "interface", ws:file("service"), "--json" }
	for _, a in ipairs(args or {}) do
		table.insert(cmd, a)
	end
	local out = shell.run(cmd, { check = true })
	return json.decode(out.stdout)
end

local function keys(derived)
	local out = {}
	for _, prompt in ipairs(derived.prompts) do
		out[#out + 1] = prompt.key
	end
	return table.concat(out, ",")
end

-- ── the control: what the archetype asks when the caller knows nothing ──

prova.test("with no answers, every default-path prompt is derived", {
	proves = "the negative control for everything below. Without it, a probe that "
		.. "silently returned nothing would make every narrowing proof pass while proving "
		.. "the opposite of what it claims.",
}, function(t)
	local ws = t:use(ws_fixture)
	t:expect(keys(derive(t, ws))):equals("service_name,github_visibility,messaging")
end)

-- ── narrowing: answers remove what no longer needs asking ──────────

prova.test("an answered prompt drops out of the derived interface", {
	covers = "docs/plans/dynamic-interface.md#describe-accepts-answers",
	proves = "the question a form generator actually has is 'what must I still ask?', "
		.. "not 'what can this archetype be asked?'. Studio supplies the GitHub settings "
		.. "from its own configuration; a form that asks for them anyway is asking the "
		.. "user to re-enter something the render will overwrite.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = derive(t, ws, { "-a", "github_visibility=public" })
	t:expect(keys(derived), "the supplied key is gone, the rest remain")
		:equals("service_name,messaging")
end)

prova.test("an answer file narrows the same way as -a", {
	proves = "the two answer channels have to agree, or a client that keeps its defaults "
		.. "in a file gets a different form from one that passes flags.",
}, function(t)
	local ws = t:use(ws_fixture)
	local file = ws:write("supplied.yaml", "github_visibility: public\nservice_name: billing\n")
	t:expect(keys(derive(t, ws, { "-A", file }))):equals("messaging")
end)

-- ── the conditional collapse ───────────────────────────────────────

prova.test("an answer that selects a branch reveals that branch, with no exploration", {
	covers = "docs/plans/dynamic-interface.md#describe-accepts-answers",
	proves = "this is the same parameter solving the harder problem. Exploration returns "
		.. "every branch with `appears_when` predicates and asks each client to build an "
		.. "evaluator for them; answering `messaging=kafka` returns the ONE interface that "
		.. "applies, which is what a wizard rendering page two actually wants.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = derive(t, ws, { "-a", "messaging=kafka" })
	t:expect(keys(derived), "the branch's prompt appears without --explore")
		:equals("service_name,github_visibility,kafka_topic")
end)

prova.test("answering the other way keeps the branch hidden", {
	proves = "the paired control. `messaging=kafka` revealing kafka_topic only means "
		.. "something if `messaging=none` does not — otherwise the prompt was unconditional "
		.. "and the proof above measured nothing.",
}, function(t)
	local ws = t:use(ws_fixture)
	t:expect(keys(derive(t, ws, { "-a", "messaging=none" })))
		:equals("service_name,github_visibility")
end)

prova.test("a switch gates the derived interface the way it gates the render", {
	proves = "switches hide prompts exactly as answers do, and a form derived without "
		.. "them describes a render nobody is going to run. `interface` accepted neither "
		.. "before this — the flags existed on `render` and were simply never declared "
		.. "here, so the probe was always answering a question no caller had asked.",
}, function(t)
	local ws = t:use(ws_fixture)
	t:expect(keys(derive(t, ws, { "-s", "ci" })), "the switched prompt joins")
		:equals("service_name,github_visibility,messaging,ci_runner")
end)

-- ── the artifact a caller fills in ─────────────────────────────────

prova.test("the answers template asks only for what is left", {
	proves = "the template is the headless mirror of the form, so it has to narrow with "
		.. "it. Emitting a key the caller just supplied invites them to answer it twice "
		.. "and disagree with themselves.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "interface", ws:file("service"), "--answers-template",
		"-a", "github_visibility=public",
	}, { check = true })
	t:expect(out.stdout, "still asks for what is unknown"):contains("service_name")
	t:expect(out.stdout, "and not for what was supplied"):never():contains("github_visibility:")
end)

-- ── the wizard loop this exists for ────────────────────────────────

prova.test("a wizard walks a conditional archetype by describing again with what it has", {
	covers = "docs/plans/dynamic-interface.md#describe-accepts-answers",
	proves = "this is progressive disclosure, and it needs no session. Describe, render "
		.. "the step, collect, describe AGAIN with those answers, render the next step — "
		.. "each round returns the interface that actually applies, so a wizard paginates "
		.. "an archetype whose later prompts did not exist until the earlier ones were "
		.. "answered. The render at the end supplies everything at once and asks nothing.",
}, function(t)
	local ws = t:use(ws_fixture)

	-- Step one: what can be asked knowing nothing.
	local step1 = derive(t, ws)
	t:expect(keys(step1)):equals("service_name,github_visibility,messaging")
	t:expect(step1.prompts[3].options ~= nil, "the branch point is a choice"):equals(true)

	-- The user picks kafka. Step two: describe again carrying that.
	local step2 = derive(t, ws, { "-a", "messaging=kafka" })
	t:expect(keys(step2), "answered keys gone, the branch's prompt revealed")
		:equals("service_name,github_visibility,kafka_topic")

	-- Everything collected: the render asks nothing at all. No `-D`, so a prompt
	-- reaching the client would have to be asked and there is no terminal to ask on.
	local render = shell.run({
		t:use(bin), "render", ws:file("service"), "--destination", ws:file("out"),
		"-a", "messaging=kafka", "-a", "service_name=billing",
		"-a", "github_visibility=public", "-a", "kafka_topic=orders",
	}, { timeout = "60s" })
	t:expect(render.code, "every prompt the two rounds surfaced was answered"):equals(0)
end)

-- ── the surfaces a remote client uses ──────────────────────────────

prova.test("MCP describe accepts answers", {
	covers = "docs/plans/dynamic-interface.md#describe-accepts-answers",
	proves = "an agent choosing answers narrows exactly as a UI does — and describe is "
		.. "contractually the CLI's `--json`, so a parameter on one and not the other is "
		.. "a fork in the contract.",
}, function(t)
	local ws = t:use(ws_fixture)
	local reqs = table.concat({
		'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"proof","version":"0"}}}',
		'{"jsonrpc":"2.0","method":"notifications/initialized"}',
		'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"describe","arguments":{"source":"'
			.. ws:file("service") .. '","answers":{"messaging":"kafka"}}}}',
		"",
	}, "\n")
	local reqfile = ws:write("describe-answers.jsonl", reqs)
	local out = shell.run({ "sh", "-c", t:use(bin) .. " mcp < " .. reqfile }, { timeout = "60s" })
	local described
	for line in string.gmatch(out.stdout, "[^\n]+") do
		local ok, msg = pcall(json.decode, line)
		if ok and type(msg) == "table" and msg.id == 2 and msg.result then
			described = json.decode(msg.result.content[1].text)
		end
	end
	t:expect(described ~= nil, "describe answered"):equals(true)
	t:expect(keys(described)):equals("service_name,github_visibility,kafka_topic")
end)

local server = prova.fixture("answer-aware-server", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	ws:write("service/archetype.yaml", 'description: "Service"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("service/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
context:prompt_select("Messaging:", "messaging", { "none", "kafka" }, { default = "none" })
if context:get("messaging") == "kafka" then
  context:prompt_text("Kafka Topic:", "kafka_topic", { default = "events" })
end
]])
	ws:write("server-config.yaml", table.concat({
		"catalog:",
		"  service:",
		'    source: "' .. ws:file("service") .. '"',
		"",
	}, "\n"))

	local port = net.free_port()
	local proc = shell.spawn({
		ctx:use(bin), "server", "--host", "127.0.0.1", "--port", tostring(port),
		"-c", ws:file("server-config.yaml"),
	})
	ctx:defer(function() proc:stop() end)
	local addr = "http://127.0.0.1:" .. port
	grpc.wait_for(addr, { timeout = "30s" })
	return { addr = addr, ws = ws }
end)

prova.test("DescribeArchetype accepts answers over the wire", {
	covers = "docs/plans/dynamic-interface.md#describe-accepts-answers",
	proves = "this is the surface Ybor Studio actually calls. Without it the parameter "
		.. "exists everywhere except the one place a remote form generator can reach, and "
		.. "a wizard resolving page two has to fall back to evaluating `appears_when` "
		.. "itself.",
}, function(t)
	local srv = t:use(server)
	local client = grpc.client(srv.addr)

	local bare = json.decode(client:call("archetect.ArchetectService/DescribeArchetype",
		{ path = "service" }).interface_json)
	t:expect(keys(bare), "control: the branch is hidden"):equals("service_name,messaging")

	local answered = json.decode(client:call("archetect.ArchetectService/DescribeArchetype",
		{ path = "service", answers_yaml = "messaging: kafka\n" }).interface_json)
	t:expect(keys(answered), "the answer resolves the branch and drops the answered key")
		:equals("service_name,kafka_topic")
end)
