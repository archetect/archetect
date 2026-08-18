-- Pages & Sections — the author's grouping intent, carried to whoever renders.
--
-- The derived interface already knows WHAT an archetype asks and IN WHAT ORDER.
-- What it cannot infer is where a long form should break and what to call each
-- break — that is intent, and only the author has it. `context:page` and
-- `context:section` are how the script says it, and they are IO messages, so
-- the same declaration paginates a wizard, headings a terminal render, and
-- structures the probe's transcript. (docs/plans/interface-pages-and-sections.md)

local workspace = require("prova.workspace")

-- The subject: one build per RUN, shared by every suite, and overridable by the
-- coverage lane's instrumented build (.prova/packages/subject/init.lua).
local bin = require("subject").bin

local ws_fixture = prova.fixture("segment-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- wizard: every container shape at once — a page with metadata, sections
	-- nested two deep, a pinned key, a branch-hidden section, an empty page,
	-- and a prompt that belongs to no container at all.
	ws:write("wizard/archetype.yaml", 'description: "Wizard"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("wizard/archetype.lua", [[
local context = Context.new()

context:page({ title = "Service Identity", help = "Who is this service?", ui = { icon = "id" } }, function(ctx)
  ctx:prompt_text("Service Name:", "service_name", { default = "orders" })
  ctx:section("Ownership", function(ctx)
    ctx:prompt_text("Team:", "team", { default = "platform" })
    ctx:section("Contact", function(ctx)
      ctx:prompt_text("Email:", "email", { default = "team@example.com" })
    end)
  end)
end)

context:page({ title = "Persistence", key = "storage" }, function(ctx)
  ctx:prompt_select("Database:", "database", { "none", "postgres" }, { default = "none" })
  if ctx:get("database") == "postgres" then
    ctx:section("Postgres", function(ctx)
      ctx:prompt_text("Schema:", "schema", { default = "public" })
    end)
  end
end)

context:page("Review", function(ctx) end)

context:prompt_confirm("Telemetry:", "telemetry", { default = true })

directory.render("contents", context)
]])
	ws:write("wizard/contents/out.toml", 'name = "{{ service_name }}"\nteam = "{{ team }}"\n')

	-- flat: not one container. The backward-compatibility control.
	ws:write("flat/archetype.yaml", 'description: "Flat"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("flat/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
context:prompt_int("Port:", "port", { default = 8080 })
directory.render("contents", context)
]])
	ws:write("flat/contents/out.toml", 'name = "{{ service_name }}"\nport = {{ port }}\n')

	-- composed: a child archetype declares its own page; the parent renders it
	-- from inside one of its own.
	ws:write("composed/archetype.yaml", table.concat({
		'description: "Composed"',
		"requires:",
		'  archetect: "3.0.0"',
		"catalog:",
		"  child:",
		'    source: "./child"',
		"",
	}, "\n"))
	ws:write("composed/archetype.lua", [[
local context = Context.new()
context:page("Parent", function(ctx)
  ctx:prompt_text("Parent Name:", "parent_name", { default = "parent" })
  catalog.render("child", context)
end)
]])
	ws:write("composed/child/archetype.yaml", 'description: "Child"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("composed/child/archetype.lua", [[
local context = Context.new()
context:page("Child Options", function(ctx)
  ctx:prompt_text("Child Name:", "child_name", { default = "child" })
end)
]])

	-- untitled: the author forgot the one required field.
	ws:write("untitled/archetype.yaml", 'description: "Untitled"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("untitled/archetype.lua", [[
local context = Context.new()
context:page({ help = "no title here" }, function(ctx) end)
]])

	-- exploding: a page body that fails partway through.
	ws:write("exploding/archetype.yaml", 'description: "Exploding"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("exploding/archetype.lua", [[
local context = Context.new()
context:page("Doomed", function(ctx)
  ctx:prompt_text("First:", "first", { default = "a" })
  error("page body blew up")
end)
]])

	-- legacy-group: the flat `group =` opt pages and sections REPLACED.
	ws:write("legacy-group/archetype.yaml", 'description: "Legacy group"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("legacy-group/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders", group = "Identity" })
]])

	return ws
end)

local function interface_json(t, ws, rel, flags)
	local cmd = { t:use(bin), "interface", ws:file(rel), "--json" }
	for _, f in ipairs(flags or {}) do
		table.insert(cmd, f)
	end
	local out = shell.run(cmd, { check = true })
	return json.decode(out.stdout)
end

-- Find a node by key anywhere in a layout tree (depth-first).
local function find_node(nodes, key)
	for _, node in ipairs(nodes or {}) do
		if node.key == key then
			return node
		end
		local hit = find_node(node.children, key)
		if hit then
			return hit
		end
	end
	return nil
end

-- The child keys of a node list, in declaration order, as "type:key".
local function shape(nodes)
	local parts = {}
	for _, node in ipairs(nodes or {}) do
		parts[#parts + 1] = node.type .. ":" .. tostring(node.key)
	end
	return table.concat(parts, ",")
end

local function prompt_named(derived, key)
	for _, prompt in ipairs(derived.prompts) do
		if prompt.key == key then
			return prompt
		end
	end
	return nil
end

-- ── the tree ───────────────────────────────────────────────────────

prova.test("a page collects the prompts declared inside it", {
	proves = "the whole point: an interface-driven client needs to know where a long "
		.. "form breaks, and only the author knows. A page is that declaration, and it "
		.. "arrives as structure — not as a naming convention a client has to guess at.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")

	local identity = find_node(derived.layout, "service_identity")
	t:expect(identity ~= nil, "the page is in the layout"):equals(true)
	t:expect(identity.type):equals("page")
	t:expect(identity.title):equals("Service Identity")
	t:expect(shape(identity.children), "its prompt, then its section"):equals(
		"prompt:service_name,section:ownership"
	)
end)

prova.test("sections nest inside pages, and inside each other", {
	proves = "prompts get composed out of shared Lua modules, so grouping has to nest "
		.. "as freely as the code does. A renderer that only understands two levels can "
		.. "flatten; one that never receives the third level cannot recover it.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")

	local ownership = find_node(derived.layout, "ownership")
	t:expect(ownership.type):equals("section")
	t:expect(shape(ownership.children)):equals("prompt:team,section:contact")

	local contact = find_node(derived.layout, "contact")
	t:expect(shape(contact.children)):equals("prompt:email")
end)

prova.test("a renderer can tell a page from a section", {
	proves = "pages and sections are the same container with different rendering intent: "
		.. "a wizard step versus a fieldset inside it. Archetect refuses to decide what "
		.. "either looks like, which only works if the distinction survives the wire.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")
	t:expect(find_node(derived.layout, "service_identity").type):equals("page")
	t:expect(find_node(derived.layout, "ownership").type):equals("section")
end)

prova.test("the layout preserves declaration order, containers and loose prompts alike", {
	proves = "script order is the archetype's authored order — the sequence a terminal "
		.. "asks in. A prompt outside every page is not an error to be normalized away; "
		.. "it renders before or after the wizard steps exactly where it was written.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")
	t:expect(shape(derived.layout)):equals(
		"page:service_identity,page:storage,page:review,prompt:telemetry"
	)
end)

-- ── metadata ───────────────────────────────────────────────────────

prova.test("a page carries the title, help, and ui a form generator needs", {
	proves = "a wizard step needs more than a break: a heading, an explanation, and "
		.. "whatever the consuming UI wants on top. `ui` is opaque and passed through "
		.. "untouched, the same contract prompts already have.",
}, function(t)
	local ws = t:use(ws_fixture)
	local identity = find_node(interface_json(t, ws, "wizard").layout, "service_identity")
	t:expect(identity.title):equals("Service Identity")
	t:expect(identity.help):equals("Who is this service?")
	t:expect(identity.ui.icon):equals("id")
end)

prova.test("a segment key defaults to a slug of its title and can be pinned", {
	proves = "a wizard routes on keys and displays titles. Titles get reworded; if the "
		.. "key rode along, saved progress and deep links would break on a copy edit. "
		.. "The default is a convenience — an author who cares pins one.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")
	-- derived: "Service Identity" -> service_identity
	t:expect(find_node(derived.layout, "service_identity") ~= nil):equals(true)
	-- pinned: title "Persistence", key "storage"
	local storage = find_node(derived.layout, "storage")
	t:expect(storage ~= nil, "the pinned key wins"):equals(true)
	t:expect(storage.title):equals("Persistence")
	t:expect(find_node(derived.layout, "persistence"), "the slug is not also present"):equals(nil)
end)

prova.test("a page with nothing in it still appears", {
	proves = "the tree is built from what the author declared, not inferred from prompt "
		.. "membership. A review or confirmation step legitimately asks nothing, and a "
		.. "wizard that silently dropped it would lose a screen.",
}, function(t)
	local ws = t:use(ws_fixture)
	local review = find_node(interface_json(t, ws, "wizard").layout, "review")
	t:expect(review ~= nil, "the empty page survives"):equals(true)
	t:expect(#review.children):equals(0)
end)

prova.test("a page without a title is a script error naming the verb", {
	proves = "the title is the one thing a container cannot be rendered without. Failing "
		.. "at the call site beats emitting a nameless step a UI has to invent a label for.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("untitled"),
		"--destination", ws:file("untitled-out"), "--headless", "-D",
	})
	t:expect(out.code, "a missing title must not be silently tolerated"):never():equals(0)
	t:expect(out.stderr, "names the verb"):contains("page")
	t:expect(out.stderr, "names what is missing"):contains("title")
end)

-- ── the breadcrumb: the interactive half ───────────────────────────

prova.test("each prompt carries the containers it was asked inside", {
	proves = "the tree serves the batch client, which gets everything at once. The "
		.. "interactive client gets one envelope at a time and would otherwise have no "
		.. "idea where it is — the breadcrumb is how it says 'Step 2 · Ownership'.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard")

	local email = prompt_named(derived, "email")
	t:expect(#email.segments, "page > section > section"):equals(3)
	t:expect(email.segments[1].kind):equals("page")
	t:expect(email.segments[1].key):equals("service_identity")
	t:expect(email.segments[2].key):equals("ownership")
	t:expect(email.segments[3].title):equals("Contact")

	local telemetry = prompt_named(derived, "telemetry")
	t:expect(telemetry.segments == nil or #telemetry.segments == 0,
		"a prompt in no container carries no breadcrumb"):equals(true)
end)

-- ── backward compatibility ─────────────────────────────────────────

prova.test("an archetype that declares no containers gets a flat layout, not a missing one", {
	proves = "every existing archetype is this case. A client should not need two code "
		.. "paths — 'has layout' and 'has to fall back to prompts' — so the flat script "
		.. "still describes itself as a layout, just one with no containers in it.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "flat")
	t:expect(shape(derived.layout)):equals("prompt:service_name,prompt:port")
	t:expect(#derived.prompts, "the flat prompt list is unchanged"):equals(2)
end)

prova.test("containers change nothing about what a render produces", {
	proves = "grouping is presentation. If declaring a page could alter the generated "
		.. "tree, authors would have to choose between a good form and correct output — "
		.. "and every existing archetype adopting pages would be a behavior change.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("wizard"),
		"--destination", ws:file("wizard-out"), "--headless", "-D",
	}, { check = true })
	t:expect(out.code):equals(0)
	t:expect(ws:read("wizard-out/out.toml")):contains('name = "orders"')
	t:expect(ws:read("wizard-out/out.toml")):contains('team = "platform"')
end)

prova.test("the flat `group` opt is a script error naming its replacement", {
	proves = "`group` was a label carried to clients that nothing rendered — a section "
		.. "with no extent, no nesting, and no way to hold anything. Sections do the job "
		.. "properly, so two ways to say one thing would be two things to learn and one "
		.. "of them a dead end. Removed rather than deprecated because it is ERRORING "
		.. "that makes the replacement discoverable: a silently dropped `group` ships an "
		.. "ungrouped form that looks deliberate.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("legacy-group"),
		"--destination", ws:file("legacy-group-out"), "--headless", "-D",
	})
	t:expect(out.code, "a removed option must not be silently ignored"):never():equals(0)
	t:expect(out.stderr, "says what happened"):contains("no longer supported")
	t:expect(out.stderr, "points at the replacement"):contains("context:section")
end)

-- ── exploration, composition, failure ──────────────────────────────

prova.test("exploration folds branch-hidden containers into one tree", {
	proves = "a section behind a select is exactly the case a wizard most needs mapped — "
		.. "show/hide on an earlier answer. Exploration already finds the prompts; the "
		.. "tree has to absorb the branch rather than fork into two layouts.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "wizard", { "--explore" })

	local postgres = find_node(derived.layout, "postgres")
	t:expect(postgres ~= nil, "the hidden section is discovered"):equals(true)
	t:expect(postgres.type):equals("section")
	t:expect(shape(postgres.children)):equals("prompt:schema")

	local storage = find_node(derived.layout, "storage")
	t:expect(shape(storage.children), "merged into the page it belongs to"):equals(
		"prompt:database,section:postgres"
	)
	t:expect(shape(derived.layout), "no duplicate pages appear"):equals(
		"page:service_identity,page:storage,page:review,prompt:telemetry"
	)
end)

prova.test("a rendered child's pages nest inside the parent's", {
	proves = "composition descends through containers the same way it descends through "
		.. "prompts. A component rendered inside a parent's page is part of that step; "
		.. "hoisting it to the top level would scramble the author's sequence.",
}, function(t)
	local ws = t:use(ws_fixture)
	local derived = interface_json(t, ws, "composed")
	local parent = find_node(derived.layout, "parent")
	t:expect(parent ~= nil):equals(true)
	t:expect(shape(parent.children)):equals("prompt:parent_name,page:child_options")
	local child = find_node(derived.layout, "child_options")
	t:expect(shape(child.children)):equals("prompt:child_name")
end)

prova.test("a page body that fails fails the render, and the probe says so", {
	proves = "a container must not swallow an error to keep its own stream tidy. The "
		.. "probe closes the segment on the way out so the transcript stays balanced, "
		.. "but the failure is the result.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("exploding"),
		"--destination", ws:file("exploding-out"), "--headless", "-D",
	})
	t:expect(out.code):never():equals(0)
	t:expect(out.stderr):contains("page body blew up")

	local derived = interface_json(t, ws, "exploding")
	t:expect(derived.completed):equals(false)
	t:expect(derived.coverage):equals("partial")
	local doomed = find_node(derived.layout, "doomed")
	t:expect(doomed ~= nil, "the mapped prefix keeps its container"):equals(true)
	t:expect(shape(doomed.children)):equals("prompt:first")
end)

-- ── the CLI: segments as terminal headings ─────────────────────────

prova.test("a terminal render announces each container as it is entered", {
	proves = "the CLI has always had the same problem in miniature — a wall of prompts "
		.. "with no sense of place. Because containers are IO messages, the declaration "
		.. "that paginates a wizard headings a terminal run for free, and archetypes "
		.. "stop hand-rolling banner lines to fake it.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		t:use(bin), "render", ws:file("wizard"),
		"--destination", ws:file("wizard-cli"), "--headless", "-D",
	}, { check = true })

	local stderr = out.stderr
	t:expect(stderr, "the page title"):contains("Service Identity")
	t:expect(stderr, "the page help"):contains("Who is this service?")
	t:expect(stderr, "the section title"):contains("Ownership")

	local page_at = string.find(stderr, "Service Identity", 1, true)
	local section_at = string.find(stderr, "Ownership", 1, true)
	local later_at = string.find(stderr, "Persistence", 1, true)
	t:expect(page_at ~= nil and section_at ~= nil and later_at ~= nil):equals(true)
	t:expect(page_at < section_at, "headings arrive in script order"):equals(true)
	t:expect(section_at < later_at, "and the next page follows"):equals(true)
end)

-- ── the artifacts a caller actually consumes ───────────────────────

prova.test("the answers template groups its keys by container", {
	proves = "the answers file is the headless mirror of the form. If the form has "
		.. "sections and the file is an undifferentiated list, a human filling in thirty "
		.. "keys loses exactly the context the author bothered to declare.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run(
		{ t:use(bin), "interface", ws:file("wizard"), "--answers-template" },
		{ check = true }
	)
	t:expect(out.stdout, "the page is a heading in the file"):contains("Service Identity")
	t:expect(out.stdout, "so is the section"):contains("Ownership")

	local heading_at = string.find(out.stdout, "Service Identity", 1, true)
	local key_at = string.find(out.stdout, "service_name:", 1, true)
	t:expect(heading_at < key_at, "the heading precedes the keys it covers"):equals(true)

	-- Still a valid answers file: it must round-trip to a zero-prompt render.
	local template = ws:write("wizard-answers.yaml", out.stdout)
	local render = shell.run({
		t:use(bin), "render", ws:file("wizard"),
		"--destination", ws:file("wizard-answered"),
		"--headless", "-A", template,
	})
	t:expect(render.code, "headings are comments, not content"):equals(0)
	t:expect(ws:read("wizard-answered/out.toml")):contains('name = "orders"')
end)

prova.test("the human summary shows the structure, not a flat list", {
	proves = "`archetect interface` is how an author checks what they just declared. A "
		.. "flat list would make the containers invisible in the one place they are most "
		.. "cheaply verified.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({ t:use(bin), "interface", ws:file("wizard") }, { check = true })
	t:expect(out.stdout):contains("Service Identity")
	t:expect(out.stdout):contains("Ownership")
	t:expect(out.stdout):contains("service_name")
end)

prova.test("MCP describe carries the layout", {
	proves = "an agent choosing answers benefits from the same grouping a human does — "
		.. "and describe is contractually the CLI's `--json`. A field the CLI has and MCP "
		.. "does not is a fork in the contract.",
}, function(t)
	local ws = t:use(ws_fixture)
	local reqs = table.concat({
		'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"proof","version":"0"}}}',
		'{"jsonrpc":"2.0","method":"notifications/initialized"}',
		'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"describe","arguments":{"source":"'
			.. ws:file("wizard") .. '"}}}',
		"",
	}, "\n")
	local reqfile = ws:write("describe-requests.jsonl", reqs)
	local out = shell.run({ "sh", "-c", t:use(bin) .. " mcp < " .. reqfile }, { timeout = "60s" })
	local described
	for line in string.gmatch(out.stdout, "[^\n]+") do
		local ok, msg = pcall(json.decode, line)
		if ok and type(msg) == "table" and msg.id == 2 then
			described = json.decode(msg.result.content[1].text)
		end
	end
	t:expect(described ~= nil, "describe answered"):equals(true)
	t:expect(shape(described.layout)):equals(
		"page:service_identity,page:storage,page:review,prompt:telemetry"
	)
end)

-- Driving a STATEFUL MCP session needs request 3 sent only after response 2
-- arrives. Piping every request at once races: the server dispatches the calls
-- concurrently and `respond` can reach the session lock before `render` has
-- stored anything, which reads as "No active render session". prova's Process
-- handle has no stdin, so the exchange gets a small co-process driver.
local mcp_driver = prova.fixture("mcp-session-driver", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	return ws:write("drive_session.py", [==[
import json, subprocess, sys

binary, source, destination = sys.argv[1], sys.argv[2], sys.argv[3]
proc = subprocess.Popen([binary, "mcp"], stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, text=True, bufsize=1)

def send(obj):
    proc.stdin.write(json.dumps(obj) + "\n")
    proc.stdin.flush()

def await_id(want):
    for line in proc.stdout:
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("id") == want:
            return message
    raise SystemExit("stream ended before id %s" % want)

send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
      "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                 "clientInfo": {"name": "proof", "version": "0"}}})
await_id(1)
send({"jsonrpc": "2.0", "method": "notifications/initialized"})

send({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
      "params": {"name": "render",
                 "arguments": {"source": source, "destination": destination}}})
first = await_id(2)

send({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
      "params": {"name": "respond", "arguments": {"value": "svc"}}})
second = await_id(3)

proc.stdin.close()
proc.kill()
print(json.dumps([json.loads(r["result"]["content"][0]["text"]) for r in (first, second)]))
]==])
end)

prova.test("an MCP agent answering one prompt at a time is told where it is", {
	requires = { "python3" },
	proves = "`describe` gives a batch client the whole tree, but an agent driving a live "
		.. "render sees one envelope per turn and would otherwise have no idea a page "
		.. "existed. This is the interactive half, and MCP is the surface that has to "
		.. "assemble it itself — a gRPC client gets the raw Begin/End messages and keeps "
		.. "its own stack, while the MCP session must carry one ACROSS exchanges: the "
		.. "section below opens during the first turn and is still open on the second.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = shell.run({
		"python3", t:use(mcp_driver), t:use(bin), ws:file("wizard"), ws:file("mcp-render"),
	}, { timeout = "60s", check = true })

	local turns = json.decode(out.stdout)
	local first, second = turns[1], turns[2]

	t:expect(first.status, "the render stopped to ask"):equals("prompting")
	t:expect(first.prompt.key):equals("service_name")
	t:expect(#first.prompt.segments, "inside the page"):equals(1)
	t:expect(first.prompt.segments[1].kind):equals("page")
	t:expect(first.prompt.segments[1].title):equals("Service Identity")

	t:expect(second.status, "and asked again"):equals("prompting")
	t:expect(second.prompt.key):equals("team")
	t:expect(#second.prompt.segments, "page > section, across two exchanges"):equals(2)
	t:expect(second.prompt.segments[1].key):equals("service_identity")
	t:expect(second.prompt.segments[2].key):equals("ownership")
end)

-- ── over the wire ──────────────────────────────────────────────────

-- The wire proofs get their own workspace and server: this file's other
-- fixtures are shared, and a server holds its tree for the file's lifetime.
local server = prova.fixture("segment-server", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	ws:write("wizard/archetype.yaml", 'description: "Wizard"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("wizard/archetype.lua", [[
local context = Context.new()
context:page({ title = "Service Identity", help = "Who is this service?" }, function(ctx)
  ctx:prompt_text("Service Name:", "service_name", { default = "orders" })
  ctx:section("Ownership", function(ctx)
    ctx:prompt_text("Team:", "team", { default = "platform" })
  end)
end)
directory.render("contents", context)
]])
	ws:write("wizard/contents/out.toml", 'name = "{{ service_name }}"\n')
	ws:write("server-config.yaml", table.concat({
		"catalog:",
		"  wizard:",
		'    source: "' .. ws:file("wizard") .. '"',
		"",
	}, "\n"))
	ws:write("server-home/.keep", "")
	ws:write("client-home/.keep", "")

	local port = net.free_port()
	local proc = shell.spawn({
		ctx:use(bin), "server", "--host", "127.0.0.1", "--port", tostring(port),
		"-c", ws:file("server-config.yaml"),
	}, { cwd = ws:file("server-home") })
	ctx:defer(function() proc:stop() end)

	local addr = "http://127.0.0.1:" .. port
	grpc.wait_for(addr, { timeout = "30s" })
	return { ws = ws, addr = addr }
end)

prova.test("a connected client sees the container headings mid-render", {
	proves = "Studio's live session is the interactive half of this feature. If pages "
		.. "only existed in the describe payload, a client that chose to stream would be "
		.. "back to an undifferentiated prompt sequence — the exact problem pages fix.",
}, function(t)
	local srv = t:use(server)
	local home = srv.ws:file("client-home")
	local out = shell.run({
		t:use(bin), "connect", srv.addr, "wizard", "--destination", ".", "-D",
	}, { cwd = home, timeout = "120s", check = true })

	local stream = out.stderr .. out.stdout
	t:expect(stream, "the page crossed the wire"):contains("Service Identity")
	t:expect(stream, "with its help text"):contains("Who is this service?")
	t:expect(stream, "and the section"):contains("Ownership")
	t:expect(fs.exists(home .. "/out.toml"), "and the render still landed"):equals(true)
end)

prova.test("DescribeArchetype serves the layout to a remote form generator", {
	proves = "the RPC is how Studio renders a form without running anything. It answers "
		.. "with the probe result verbatim, so the layout has to be in that payload or "
		.. "the wizard has no steps.",
}, function(t)
	local srv = t:use(server)
	local client = grpc.client(srv.addr)
	local reply = client:call("archetect.ArchetectService/DescribeArchetype", { path = "wizard" })
	local payload = json.decode(reply.interface_json)
	t:expect(shape(payload.layout)):equals("page:service_identity")
	local page = find_node(payload.layout, "service_identity")
	t:expect(shape(page.children)):equals("prompt:service_name,section:ownership")
end)

prova.test("a wizard renders a whole archetype in one exchange, with no prompts at all", {
	proves = "the round-trip cost a wizard actually pays. `describe` hands the client the "
		.. "layout AND every prompt up front, so a paginated UI can collect the lot and "
		.. "submit them with the render — one describe, one render, zero prompt exchanges. "
		.. "This is the flow Studio wants, and proving it end-to-end is what says the "
		.. "streaming session is an option rather than an obligation.",
}, function(t)
	local srv = t:use(server)
	local home = srv.ws:file("wizard-client")
	srv.ws:write("wizard-client/.keep", "")

	-- Step 1: ask the server what to render. A wizard walks `layout` to lay out
	-- its steps; here we walk it to build the answer set, which exercises the
	-- same traversal.
	local client = grpc.client(srv.addr)
	local reply = client:call("archetect.ArchetectService/DescribeArchetype", { path = "wizard" })
	local payload = json.decode(reply.interface_json)

	local answers, keys = {}, {}
	local function collect(nodes)
		for _, node in ipairs(nodes or {}) do
			if node.type == "prompt" then
				keys[#keys + 1] = node.key
				answers[#answers + 1] = node.key .. ": wizard-" .. node.key
			else
				collect(node.children)
			end
		end
	end
	collect(payload.layout)
	t:expect(#keys, "the layout named every prompt to collect"):equals(2)

	-- Step 2: render with the collected answers. Deliberately NO `-D`: if any
	-- prompt were unanswered the client would have to ask, so a clean exit is
	-- itself the evidence that nothing needed asking.
	local file = srv.ws:write("wizard-answers.yaml", table.concat(answers, "\n") .. "\n")
	local out = shell.run({
		t:use(bin), "connect", srv.addr, "wizard", "--destination", ".", "-A", file,
	}, { cwd = home, timeout = "60s" })
	t:expect(out.code, "every prompt was pre-answered"):equals(0)
	t:expect(srv.ws:read("wizard-client/out.toml"),
		"the wizard's values reached the render"):contains("wizard-service_name")
end)

prova.test("without the collected answers, the same render does NOT produce them", {
	proves = "the negative control for the proof above. Both prompts carry defaults, so a "
		.. "render that quietly fell back to them would exit 0 and write a plausible file "
		.. "— and the wizard proof would be green while proving nothing about whether the "
		.. "answers crossed the wire at all.",
}, function(t)
	local srv = t:use(server)
	local home = srv.ws:file("wizard-control")
	srv.ws:write("wizard-control/.keep", "")

	local out = shell.run({
		t:use(bin), "connect", srv.addr, "wizard", "--destination", ".", "-D",
	}, { cwd = home, timeout = "60s", check = true })
	t:expect(out.code):equals(0)
	t:expect(srv.ws:read("wizard-control/out.toml"),
		"defaults, not the wizard's values"):never():contains("wizard-service_name")
	t:expect(srv.ws:read("wizard-control/out.toml")):contains("orders")
end)

-- ── the authoring surface teaches itself ──────────────────────────

prova.test("introspect teaches page and section", {
	proves = "the scripting API is asked, never guessed at — `introspect` and the IDE read "
		.. "the same LuaCATS stub. A verb that exists in the runtime but not the stub is "
		.. "undiscoverable to both an author and an agent.",
}, function(t)
	local out = shell.run({ t:use(bin), "introspect", "Context:page" }, { check = true })
	t:expect(out.stdout):contains("Context:page")
	t:expect(out.stdout, "the section verb too"):contains("Context:section")
	t:expect(out.stdout, "and the options table"):contains("SegmentOpts")

	local opts = shell.run({ t:use(bin), "introspect", "SegmentOpts" }, { check = true })
	t:expect(opts.stdout, "the required field"):contains("title")
	t:expect(opts.stdout, "and the routing key"):contains("key")
end)

prova.test("learn prompts documents pages and sections", {
	proves = "`learn` is the first thing an agent reads before authoring. A feature that "
		.. "only exists in a plan document is a feature nobody will use.",
}, function(t)
	local out = shell.run({ t:use(bin), "learn", "prompts" }, { check = true })
	t:expect(out.stdout):contains("context:page")
	t:expect(out.stdout):contains("context:section")
	t:expect(out.stdout, "and the derived-interface field they produce"):contains("layout")
end)

-- ── the executable backlog ─────────────────────────────────────────

prova.test("an INTERACTIVE archetype's page is answered in one exchange", {
	promises = "the original premise — 'a wizard's Next costs N round trips' — turned out "
		.. "to be wrong for most archetypes, and the two proofs above are why: `describe` "
		.. "hands a client the layout AND every prompt, so a batch-classified archetype is "
		.. "collected up front and rendered with zero prompt exchanges. What is left is "
		.. "the honest case: an archetype whose prompt set depends on its own answers "
		.. "classifies `interactive`, cannot be collected in advance, and still pays one "
		.. "round trip per field.\n\n"
		.. "Batching THAT needs the server to know a page's prompts before executing it, "
		.. "and the only way to learn them is to run the body — so the shape is two-pass: "
		.. "execute the page against a recording driver, send the envelopes as one batch, "
		.. "take a keyed answer map back, then re-execute with those answers seeded. The "
		.. "hazard is the reason this is not built yet: a page body that renders a child, "
		.. "shells out, or reads the clock would run twice. Suppressing writes and exec on "
		.. "pass one (the probe already does) covers most of it and not `catalog.render`. "
		.. "Needs a decision on that hazard, a ClientMessage variant for the answer map, "
		.. "and a proof that counts exchanges rather than asserting an exit code.",
}, function(t)
	local srv = t:use(server)
	local home = srv.ws:file("wizard-interactive")
	srv.ws:write("wizard-interactive/.keep", "")
	local out = shell.run({
		t:use(bin), "connect", srv.addr, "wizard", "--destination", ".", "-D",
		"--page-at-a-time",
	}, { cwd = home, timeout = "60s" })
	t:expect(out.code):equals(0)
end)
