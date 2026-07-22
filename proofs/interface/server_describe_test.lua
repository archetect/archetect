-- Dynamic-interface phase 6: DescribeArchetype over gRPC — a portal asks the
-- SERVER for the form. (docs/plans/dynamic-interface.md §7.)

local workspace = require("prova.workspace")

local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

-- A server whose catalog holds one probeable archetype.
local server = prova.fixture("describe-server", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	ws:write("flat/archetype.yaml", 'description: "Flat"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("flat/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders", pattern = "^[a-z][a-z0-9-]*$" })
context:prompt_select("Persistence:", "persistence", { "none", "pg" }, { default = "none" })
if context:get("persistence") == "pg" then
  context:prompt_text("Database Name:", "db_name", { default = "app" })
end
if archetype.switches.is_enabled("ci") then context:set("ci", true) end
]])
	ws:write("server-config.yaml", table.concat({
		"catalog:",
		"  flat:",
		'    source: "' .. ws:file("flat") .. '"',
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

prova.test("DescribeArchetype serves the derived interface over gRPC", function(t)
	local srv = t:use(server)
	local client = grpc.client(srv.addr)
	local reply = client:call("archetect.ArchetectService/DescribeArchetype", { path = "flat" })
	local derived = prova.parse.json(reply.interface_json)
	t:expect(#derived.prompts):equals(2)
	t:expect(derived.prompts[1].key):equals("service_name")
	t:expect(derived.prompts[1].pattern):equals("^[a-z][a-z0-9-]*$")
	t:expect(derived.switches[1], "switch recording crosses the wire"):equals("ci")
	t:expect(derived.coverage):equals("default-path")
end)

prova.test("DescribeArchetype explores branches when asked", function(t)
	local srv = t:use(server)
	local client = grpc.client(srv.addr)
	local reply = client:call("archetect.ArchetectService/DescribeArchetype", { path = "flat", explore = true })
	local derived = prova.parse.json(reply.interface_json)
	t:expect(derived.mode):equals("batch")
	local keys = {}
	for _, p in ipairs(derived.prompts) do keys[#keys + 1] = p.key end
	t:expect(table.concat(keys, ","), "branch prompt found"):contains("db_name")
end)

prova.test("DescribeArchetype rejects a non-leaf path with a real status", function(t)
	local srv = t:use(server)
	local client = grpc.client(srv.addr)
	local result = client:call_status("archetect.ArchetectService/DescribeArchetype", { path = "nope" })
	t:expect(result.ok):equals(false)
	t:expect(result.code):equals("FailedPrecondition")
end)
