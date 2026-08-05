-- Action resolution — WHICH archetype a server render targets, and how honestly.
--
-- Three rules, one invariant: a caller can never be handed an archetype other than
-- the one it named.
--
--   1. `archetect server <action>` resolves its action at STARTUP through the same
--      walker `DescribeArchetype` uses, and refuses to start when it does not name a
--      renderable leaf — never binds a port it can only serve wrong answers on.
--   2. `archetect connect <endpoint> <path>` sends the path over the wire
--      (Initialize.catalog_path), and the server resolves it strongly — including
--      descent through source-backed sub-catalogs. Anything describable is renderable.
--   3. With no path from either side, only an UNAMBIGUOUS default renders: a catalog
--      entry named "default", or a single-leaf catalog. Anything else is an error
--      naming the entries — never the first entry by declaration order.

local workspace = require("prova.workspace")

local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

-- Two archetypes with distinct markers, addressable both at the top level and through
-- a source-backed (script-less) sub-catalog — the shape only the strong walker resolves.
local function write_archetype(ws, name)
	ws:write(name .. "/archetype.yaml", 'description: "' .. name .. '"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write(name .. "/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
directory.render("contents", context)
]])
	ws:write(name .. "/contents/{{ service_name }}/MARKER.md", string.upper(name) .. " {{ service_name }}\n")
end

local suite = prova.fixture("action-resolution-ws", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	write_archetype(ws, "alpha")
	write_archetype(ws, "beta")

	-- A script-less catalog manifest: entry "sub" resolves INTO this via its source.
	ws:write("sub/archetype.yaml", table.concat({
		'description: "Sub catalog"',
		"requires:",
		'  archetect: "3.0.0"',
		"catalog:",
		"  beta:",
		'    source: "' .. ws:file("beta") .. '"',
		"",
	}, "\n"))

	-- alpha is deliberately FIRST everywhere: the legacy fallback rendered it whenever
	-- resolution failed, which is exactly the bug these proofs pin down.
	ws:write("two-leaves.yaml", table.concat({
		"catalog:",
		"  alpha:",
		'    source: "' .. ws:file("alpha") .. '"',
		"  beta:",
		'    source: "' .. ws:file("beta") .. '"',
		"",
	}, "\n"))
	ws:write("with-sub.yaml", table.concat({
		"catalog:",
		"  alpha:",
		'    source: "' .. ws:file("alpha") .. '"',
		"  sub:",
		'    source: "' .. ws:file("sub") .. '"',
		"",
	}, "\n"))
	ws:write("solo.yaml", table.concat({
		"catalog:",
		"  solo:",
		'    source: "' .. ws:file("alpha") .. '"',
		"",
	}, "\n"))
	ws:write("preset.yaml", table.concat({
		"catalog:",
		"  preset:",
		'    source: "' .. ws:file("alpha") .. '"',
		"    answers:",
		'      service_name: "presets"',
		"",
	}, "\n"))
	return ws
end)

local function start_server(ctx, ws, config, name, action)
	local home = ws:file(name .. "-server-home")
	ws:write(name .. "-server-home/.keep", "")
	local port = net.free_port()
	local argv = { ctx:use(bin), "server", "--host", "127.0.0.1", "--port", tostring(port), "-c", ws:file(config) }
	if action then
		table.insert(argv, action)
	end
	local proc = shell.spawn(argv, { cwd = home })
	ctx:defer(function() proc:stop() end)
	local addr = "http://127.0.0.1:" .. port
	grpc.wait_for(addr, { timeout = "30s" })
	return { addr = addr, home = home }
end

local function client_home(ws, name)
	ws:write(name .. "/.keep", "")
	return ws:file(name)
end

prova.test("a server refuses to start on an unresolvable action", {
	proves = "an action typo (or a path the walker cannot reach) must fail startup loudly — "
		.. "a server that binds its port anyway serves every caller the wrong archetype.",
}, function(t)
	local ws = t:use(suite)
	local result = shell.run({
		t:use(bin), "server", "--host", "127.0.0.1", "--port", tostring(net.free_port()),
		"-c", ws:file("two-leaves.yaml"), "does/not/exist",
	}, { timeout = "30s", check = false, merge_stderr = true })

	t:expect(result:ok(), "startup must fail"):equals(false)
	t:expect(result.stdout, "naming the action it could not resolve"):contains("does/not/exist")
end)

prova.test("the server's action is the render target, not the first catalog entry", {
	proves = "`archetect server beta` must serve beta. Before this, the action argument was "
		.. "accepted and ignored, and every pathless render fell back to the first entry — "
		.. "a caller could be handed a different archetype than the operator deployed.",
}, function(t)
	local ws = t:use(suite)
	local srv = start_server(t, ws, "two-leaves.yaml", "action-beta", "beta")
	local home = client_home(ws, "client-action-beta")

	shell.run({ t:use(bin), "connect", srv.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = true })

	t:expect(fs.exists(home .. "/orders/MARKER.md"), "a render happened"):equals(true)
	t:expect(fs.read(home .. "/orders/MARKER.md"), "and it is the named archetype"):contains("BETA")
end)

prova.test("a client-named catalog path renders that leaf — anything describable is renderable", {
	proves = "`connect <endpoint> sub/beta` sends the path over the wire and the server resolves "
		.. "it with the same strong walker DescribeArchetype uses — including descent through a "
		.. "source-backed sub-catalog. Describe and render can no longer disagree about a path.",
}, function(t)
	local ws = t:use(suite)
	local srv = start_server(t, ws, "with-sub.yaml", "path-addressing")
	local home = client_home(ws, "client-path")

	-- The same path must first be describable...
	local client = grpc.client(srv.addr)
	local described = client:call("archetect.ArchetectService/DescribeArchetype", { path = "sub/beta" })
	t:expect(json.decode(described.interface_json).prompts ~= nil, "the path describes"):is_true()

	-- ...and then renderable, by name, from the client.
	shell.run({ t:use(bin), "connect", srv.addr, "sub/beta", "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = true })

	t:expect(fs.exists(home .. "/orders/MARKER.md"), "the leaf rendered"):equals(true)
	t:expect(fs.read(home .. "/orders/MARKER.md"), "and it is the addressed one"):contains("BETA")
end)

prova.test("an ambiguous default errors instead of rendering an arbitrary entry", {
	proves = "with no path from either side and no 'default' entry, a multi-leaf catalog has no "
		.. "honest answer — the render must fail naming the available entries, not silently pick "
		.. "the first one.",
}, function(t)
	local ws = t:use(suite)
	local srv = start_server(t, ws, "two-leaves.yaml", "ambiguous-default")
	local home = client_home(ws, "client-ambiguous")

	local result = shell.run({ t:use(bin), "connect", srv.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = false, merge_stderr = true })

	t:expect(result:ok(), "an ambiguous default must not exit 0"):equals(false)
	t:expect(result.stdout, "the error names the choices"):contains("alpha")
	t:expect(fs.exists(home .. "/orders"), "and nothing was rendered"):equals(false)
end)

prova.test("a single-leaf catalog still renders as the default", {
	proves = "the ambiguity rule must not break the one-archetype-per-server pattern this suite's "
		.. "own harness uses: a catalog with exactly one leaf IS unambiguous, whatever its name.",
}, function(t)
	local ws = t:use(suite)
	local srv = start_server(t, ws, "solo.yaml", "solo-default")
	local home = client_home(ws, "client-solo")

	shell.run({ t:use(bin), "connect", srv.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = true })

	t:expect(fs.exists(home .. "/orders/MARKER.md"), "the only leaf rendered"):equals(true)
end)

prova.test("catalog entry answers apply to server renders", {
	proves = "a catalog entry's pre-configured answers are part of what the entry MEANS — the "
		.. "CLI applies them on every catalog render, so a server render through the same entry "
		.. "must too, or the two surfaces render different projects from one catalog.",
}, function(t)
	local ws = t:use(suite)
	local srv = start_server(t, ws, "preset.yaml", "entry-answers", "preset")
	local home = client_home(ws, "client-preset")

	shell.run({ t:use(bin), "connect", srv.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = true })

	t:expect(fs.exists(home .. "/presets/MARKER.md"), "the entry's answers named the tree"):equals(true)
end)
