-- Materialization — WHO owns the filesystem a render lands on.
--
-- In server mode `WriteFile`/`WriteDirectory` are server→client messages: the
-- server streams bytes and the CLIENT writes them. That is what makes a
-- Studio-style client's "working directory" a purely client-side concept.
--
-- The first proof pins that invariant. The specs below cover the effects that
-- still bypass the IO channel (`archive`, `git`, `github` call std::fs/git2
-- directly against `render_context.destination()`), and the completion payload
-- a non-filesystem client needs in order to learn what was produced.

local workspace = require("prova.workspace")

local bin = prova.fixture("archetect-bin", Scope.File, function(ctx)
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

-- Each archetype gets its own single-entry server: `connect` always renders the
-- server's `default` catalog entry (no catalog-path flag on the subcommand), so
-- selecting between archetypes means selecting between endpoints.
local function start_server(ctx, ws, name)
	ws:write(name .. "-config.yaml", table.concat({
		"catalog:",
		"  default:",
		'    source: "' .. ws:file(name) .. '"',
		"",
	}, "\n"))
	-- The server runs from its OWN cwd. A relative destination that resolved
	-- server-side would land here — that is the discriminator below.
	local home = ws:file(name .. "-server-home")
	ws:write(name .. "-server-home/.keep", "")

	local port = net.free_port()
	local proc = shell.spawn({
		ctx:use(bin), "server", "--host", "127.0.0.1", "--port", tostring(port),
		"-c", ws:file(name .. "-config.yaml"),
	}, { cwd = home })
	ctx:defer(function() proc:stop() end)

	local addr = "http://127.0.0.1:" .. port
	grpc.wait_for(addr, { timeout = "30s" })
	return { addr = addr, home = home }
end

local servers = prova.fixture("materialization-servers", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- Renders a directory whose NAME comes from an answer.
	ws:write("plain/archetype.yaml", 'description: "Plain"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("plain/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
directory.render("contents", context)
]])
	ws:write("plain/contents/{{ service_name }}/README.md", "# {{ service_name }}\n")

	-- Same, plus the conventions-based archive step. Both the source directory
	-- and the archive filename are derived from the answer — the reason
	-- archiving belongs to the script and not to the consumer.
	ws:write("packaged/archetype.yaml", 'description: "Packaged"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("packaged/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
directory.render("contents", context)
local name = context:get("service_name")
archive.zip(name, name .. ".zip")
]])
	ws:write("packaged/contents/{{ service_name }}/README.md", "# {{ service_name }}\n")

	-- Renders first, then reaches for an external system with platform
	-- credentials — the effect class that must be authorized, not ambient.
	ws:write("publishing/archetype.yaml", 'description: "Publishing"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("publishing/archetype.lua", [[
local github = require("archetect.github")
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
directory.render("contents", context)
github.create_repo("archetect-proof-org", context:get("service_name"))
]])
	ws:write("publishing/contents/{{ service_name }}/README.md", "# {{ service_name }}\n")

	-- Fails partway through, after it has already written something.
	ws:write("failing/archetype.yaml", 'description: "Failing"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("failing/archetype.lua", [[
local context = Context.new()
context:prompt_text("Service Name:", "service_name", { default = "orders" })
directory.render("contents", context)
error("deliberate mid-render failure")
]])
	ws:write("failing/contents/{{ service_name }}/README.md", "# {{ service_name }}\n")

	return {
		ws = ws,
		plain = start_server(ctx, ws, "plain"),
		packaged = start_server(ctx, ws, "packaged"),
		publishing = start_server(ctx, ws, "publishing"),
		failing = start_server(ctx, ws, "failing"),
	}
end)

-- A fresh client-side directory to render into, distinct from any server's cwd.
local function client_home(srv, name)
	srv.ws:write(name .. "/.keep", "")
	return srv.ws:file(name)
end

prova.test("a relative destination resolves on the CLIENT, not the server", {
	proves = "WriteFile/WriteDirectory are server→client messages: the client owns the "
		.. "filesystem. This is what lets a Studio client point a render at any local "
		.. "working directory without the server knowing that path exists.",
}, function(t)
	local srv = t:use(servers)
	local home = client_home(srv, "client-plain")

	shell.run({ t:use(bin), "connect", srv.plain.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = true })

	t:expect(fs.exists(home .. "/orders/README.md"), "tree lands in the client's cwd"):equals(true)
	t:expect(fs.exists(srv.plain.home .. "/orders"), "server's cwd stays clean"):equals(false)
end)

prova.test("a server-side render failure reaches the client", {
	proves = "a failed generation must never be indistinguishable from a successful one — "
		.. "the whole point of a remote render is that the caller cannot see the server's logs.",
}, function(t)
	local srv = t:use(servers)
	local home = client_home(srv, "client-failing")

	local result = shell.run({ t:use(bin), "connect", srv.failing.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = false })

	t:expect(result:ok(), "a failed render must not exit 0"):equals(false)
end)

prova.test("archive.zip materializes where the tree lives", {
	spec = "materialization: archive bypasses the IO channel — require_modules.rs:771 writes "
		.. "via std::fs against the SERVER's destination, which is never populated in server "
		.. "mode. Archiving must become a channel intent executed by whichever side holds the "
		.. "tree, while the script keeps deriving both names from answers.",
}, function(t)
	local srv = t:use(servers)
	local home = client_home(srv, "client-packaged")

	shell.run({ t:use(bin), "connect", srv.packaged.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = false })

	t:expect(fs.exists(home .. "/orders/README.md"), "tree lands client-side"):equals(true)
	t:expect(fs.exists(home .. "/orders.zip"), "so does the archive built from it"):equals(true)
end)

prova.test("completion reports the artifacts a render produced", {
	spec = "materialization: CompleteSuccess carries no payload (archetect.proto:166), so a "
		.. "client cannot learn the script-derived archive name. Ybor Studio needs exactly "
		.. "this to offer a download. Completion must carry an artifact manifest.",
}, function(t)
	local srv = t:use(servers)
	local home = client_home(srv, "client-manifest")

	local result = shell.run({ t:use(bin), "connect", srv.packaged.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = false })

	t:expect(result.stdout, "the resolved artifact name reaches the client"):contains("orders.zip")
end)

prova.test("a render needing publish is refused before it writes anything", {
	spec = "materialization: github.create_repo is ambient authority — any archetype in the "
		.. "catalog can reach a third-party system with the platform's token. A PaaS cannot "
		.. "safely host an open catalog under that rule. Publish must be a capability the "
		.. "client grants at Initialize, checked up front rather than mid-render.",
}, function(t)
	local srv = t:use(servers)
	local home = client_home(srv, "client-publish")

	local result = shell.run({ t:use(bin), "connect", srv.publishing.addr, "--destination", ".", "-D" },
		{ cwd = home, timeout = "120s", check = false })

	t:expect(result:ok(), "an ungranted capability fails the render"):equals(false)
	t:expect(fs.exists(home .. "/orders"), "refused up front — nothing was written"):equals(false)
end)
