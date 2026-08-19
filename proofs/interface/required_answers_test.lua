-- A prompt's rules bind the VALUE, not the path it arrived on.
--
-- `pattern` has always been enforced everywhere — interactive, `-a`, answer
-- files, MCP — and that is what makes it worth declaring. Two other guards were
-- not, and both failed in the direction that looks like success:
--
--   * a non-optional prompt accepted an EMPTY answer. `-a project_name=`
--     rendered modules named `-bom`, `-core` and `-server`, an empty
--     `<artifactId>`, and dropped the project at the destination root instead of
--     in its own directory. Exit 0, no warning.
--   * `min`/`max` were enforced by the terminal's validator and nowhere else, so
--     `min = 1` accepted that same empty answer. An author who writes it
--     reasonably believes they are covered, which is worse than not having it.
--
-- This is not hypothetical for the hybrid wizard drive, where a client sends an
-- empty string for a field the user tabbed past — so the wire path is proven
-- here too, not just the CLI one.
-- (docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts)

local workspace = require("prova.workspace")

local bin = require("subject").bin

local ws_fixture = prova.fixture("required-answers-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- The reported shape: the answer names the directory the whole project lands
	-- in, so an empty one does not merely produce an empty field — it collapses
	-- the tree into the destination root.
	ws:write("service/archetype.yaml", 'description: "Service"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("service/archetype.lua", [[
local context = Context.new()
context:prompt_text("Project Name:", "project_name", { min = 1 })
context:prompt_text("Nickname:", "nickname", { optional = true })
directory.render("contents", context)
]])
	ws:write("service/contents/{{ project_name }}/pom.xml", "<artifactId>{{ project_name }}</artifactId>\n")

	-- Bounds with room on both sides, so "too short" and "too long" are separable
	-- from "empty".
	ws:write("bounded/archetype.yaml", 'description: "Bounded"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("bounded/archetype.lua", [[
local context = Context.new()
context:prompt_text("Code:", "code", { min = 3, max = 8, default = "abcde" })
context:prompt_text("Alias:", "alias", { min = 3, optional = true })
context:prompt_int("Port:", "port", { min = 1024, max = 65535, default = 8080 })
directory.render("contents", context)
]])
	ws:write("bounded/contents/out.toml", 'code = "{{ code }}"\nport = {{ port }}\n')

	-- A default the prompt's own rules reject. The terminal has always refused
	-- this value; supplying it from the archetype must not be the way around.
	ws:write("baddefault/archetype.yaml", 'description: "Bad Default"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("baddefault/archetype.lua", [[
local context = Context.new()
context:prompt_text("Code:", "code", { min = 3, default = "ab" })
directory.render("contents", context)
]])
	ws:write("baddefault/contents/out.toml", 'code = "{{ code }}"\n')

	return ws
end)

local function render(t, ws, archetype, dest, args)
	local cmd = {
		t:use(bin), "render", ws:file(archetype),
		"--destination", ws:file(dest), "--headless",
	}
	for _, a in ipairs(args or {}) do
		table.insert(cmd, a)
	end
	return shell.run(cmd, { timeout = "60s" })
end

-- ── the control: the guard must not swallow legitimate renders ─────

prova.test("a real answer renders, into the directory it names", {
	proves = "the negative control for everything below. A change that refused empty "
		.. "answers by refusing answers would pass every failure proof in this file and "
		.. "break every archetype in the catalog.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = render(t, ws, "service", "ok", { "-a", "project_name=orders", "-D" })
	t:expect(out.code, "exit"):equals(0)
	t:expect(ws:read("ok/orders/pom.xml")):contains("<artifactId>orders</artifactId>")
end)

-- ── empty is not an answer to a required prompt ────────────────────

prova.test("an empty answer to a required prompt is refused, and nothing is written", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "the failure this claim was captured for. Rendering structural garbage and "
		.. "exiting 0 is the worst available outcome — a broken generation is "
		.. "indistinguishable from a good one, and the user finds out from a build "
		.. "failure in a tree they now have to unpick. Refusing before the first write "
		.. "means there is nothing to unpick.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = render(t, ws, "service", "empty", { "-a", "project_name=", "-D" })
	t:expect(out.code, "exit"):never():equals(0)
	t:expect(out.stderr, "names the key"):contains("project_name")
	t:expect(out.stderr, "says what is wrong with it"):contains("not optional")
	t:expect(fs.exists(ws:file("empty/pom.xml")), "the collapsed tree is not on disk")
		:equals(false)
end)

prova.test("an optional prompt still takes an empty answer", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "`optional` is the author saying empty is a legitimate value here, and it "
		.. "has to keep meaning that. The rule is about non-optional prompts; a guard "
		.. "that could not tell the two apart would be a blunter instrument than the bug.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = render(t, ws, "service", "opt", { "-a", "project_name=orders", "-a", "nickname=", "-D" })
	t:expect(out.code, "exit"):equals(0)
	t:expect(fs.exists(ws:file("opt/orders/pom.xml"))):equals(true)
end)

-- ── min and max bind the value, not the terminal ───────────────────

prova.test("min and max are enforced on the answer path, not only at the terminal", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "the half that is worse because it looks like it works. `pattern` "
		.. "documents itself as enforced on every input path and is; `min`/`max` sat "
		.. "beside it in the same opts table, enforced only by inquire's validator. An "
		.. "author writing `min = 1` believes they have declared a rule, and a headless "
		.. "render is precisely where nobody is watching.",
}, function(t)
	local ws = t:use(ws_fixture)

	local short = render(t, ws, "bounded", "short", { "-a", "code=ab", "-D" })
	t:expect(short.code, "below min"):never():equals(0)
	t:expect(short.stderr, "names the key"):contains("code")

	local long = render(t, ws, "bounded", "long", { "-a", "code=abcdefghij", "-D" })
	t:expect(long.code, "above max"):never():equals(0)
	t:expect(long.stderr, "names the key"):contains("code")

	local fits = render(t, ws, "bounded", "fits", { "-a", "code=abcde", "-D" })
	t:expect(fits.code, "within bounds"):equals(0)
	t:expect(ws:read("fits/out.toml")):contains('code = "abcde"')
end)

prova.test("an optional prompt with a length rule still accepts the skip", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "the one interaction between the two halves, and the branch that would "
		.. "otherwise go untested. On the answer path an empty string is the ONLY way to "
		.. "say 'skipped', so measuring it against `min` would make `optional` and `min` "
		.. "mutually exclusive — a rule about how long a value must be has nothing to say "
		.. "about there not being one.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = render(t, ws, "bounded", "skipped", { "-a", "alias=", "-D" })
	t:expect(out.code, "the skip is a skip, not a zero-length value"):equals(0)

	local short = render(t, ws, "bounded", "shortalias", { "-a", "alias=xy", "-D" })
	t:expect(short.code, "but a value that IS there is still measured"):never():equals(0)
	t:expect(short.stderr, "names the key"):contains("alias")
end)

prova.test("an int answer outside its bounds is refused the same way", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "`min`/`max` on `prompt_int` is the same declaration with the same hole — "
		.. "the terminal validated it, the answer path did not. A port prompt bounded to "
		.. "the unprivileged range is the canonical use, and `-a port=80` sailed through.",
}, function(t)
	local ws = t:use(ws_fixture)

	local low = render(t, ws, "bounded", "lowport", { "-a", "port=80", "-D" })
	t:expect(low.code, "below min"):never():equals(0)
	t:expect(low.stderr, "names the key"):contains("port")

	local ok = render(t, ws, "bounded", "goodport", { "-a", "port=9090", "-D" })
	t:expect(ok.code, "within bounds"):equals(0)
	t:expect(ws:read("goodport/out.toml")):contains("port = 9090")
end)

prova.test("a default its own prompt would reject fails at the archetype", {
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "the rule binds the value, so where it came from cannot be a way around it "
		.. "— and the terminal has always agreed: type nothing at a `min = 3` prompt and "
		.. "inquire refuses. `-D` accepting what an interactive run rejects would make "
		.. "the two drives disagree about what the archetype means, which is the one "
		.. "thing headless mode must never do.",
}, function(t)
	local ws = t:use(ws_fixture)
	local out = render(t, ws, "baddefault", "bad", { "-D" })
	t:expect(out.code, "exit"):never():equals(0)
	t:expect(out.stderr, "names the key"):contains("code")
end)

-- ── the wire path: the case the hybrid wizard actually hits ────────

-- A live session needs request 3 sent only after response 2 arrives; piping
-- every request at once races the session lock. prova's Process handle has no
-- stdin, so the exchange gets a small co-process driver.
local mcp_driver = prova.fixture("required-answers-mcp-driver", Scope.File, function(ctx)
	local ws = workspace.create(ctx)
	return ws:write("drive_session.py", [==[
import json, subprocess, sys

binary, source, destination, value = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
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
await_id(2)

send({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
      "params": {"name": "respond", "arguments": {"value": value}}})
answered = await_id(3)

proc.stdin.close()
proc.kill()
print(json.dumps(answered["result"]))
]==])
end)

prova.test("a client answering a required prompt with an empty string is refused too", {
	requires = { "python3" },
	covers = "docs/plans/dynamic-interface.md#empty-answers-satisfy-required-prompts",
	proves = "the motivating case, and the reason this could not be fixed in the answer "
		.. "parser alone. In the hybrid drive a client collects a page into a form and "
		.. "posts it back; a field the user tabbed past arrives as an empty string over "
		.. "the same channel a terminal would have refused at the keystroke. The guard "
		.. "belongs where every driver meets the script, not in one driver.",
}, function(t)
	local ws = t:use(ws_fixture)
	local driver = t:use(mcp_driver)

	local out = shell.run({
		"python3", driver, t:use(bin), ws:file("service"), ws:file("wire"), "",
	}, { timeout = "60s" })
	t:expect(out.code, "the driver completed the exchange"):equals(0)

	local result = json.decode(out.stdout)
	local text = result.content[1].text
	t:expect(text, "the empty answer is rejected, not accepted"):contains("not optional")
	t:expect(fs.exists(ws:file("wire/pom.xml")), "and nothing collapsed onto disk"):equals(false)
end)
