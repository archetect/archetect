-- The HYBRID mode: a wizard driven by re-derivation, one page at a time.
--
-- Archetect has had two ways to be driven, and each gives up something the
-- other keeps:
--
--   interactive   prompt by prompt over a live session. Handles any dynamism,
--                 because the script decides as it goes — but the client never
--                 sees more than one field ahead, so it cannot lay out a form.
--   big bang      one `describe`, the whole interface, render it as a form.
--                 Lays out beautifully — but a prompt that only exists once an
--                 earlier one is answered cannot be in it.
--
-- Answer-aware derivation makes a third shape possible: describe, render the
-- first unfinished page, collect, describe AGAIN carrying what you have, repeat.
-- Each round is an idempotent query — no session, no server state — and the
-- accumulated answers drive a final render that asks nothing.
--
-- This file proves the LOOP, not the parameter (that is answer_aware_test.lua):
-- that it terminates, that it copes with both shapes of conditional, and that
-- what it collects is exactly what the render needs.

local workspace = require("prova.workspace")

local bin = require("subject").bin

local ws_fixture = prova.fixture("wizard-loop-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- Both dependency shapes at once, which is the point:
	--
	--   CROSS-PAGE  the whole "Postgres" page exists only when Storage's answer
	--               selects it — the case the hybrid was invented for.
	--   INTRA-PAGE  `pool` appears inside the SAME page as the answer that
	--               reveals it, so that page cannot be laid out in one shot and
	--               the loop must be willing to stay on a step.
	ws:write("wizard/archetype.yaml", 'description: "Wizard"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("wizard/archetype.lua", [[
local context = Context.new()

context:page("Identity", function(ctx)
  ctx:prompt_text("Service Name:", "service_name", { default = "orders" })
end)

context:page("Storage", function(ctx)
  ctx:prompt_select("Engine:", "engine", { "none", "postgres" }, { default = "none" })
  if ctx:get("engine") == "postgres" then
    ctx:prompt_int("Pool Size:", "pool", { default = 10 })
  end
end)

if context:get("engine") == "postgres" then
  context:page("Postgres", function(ctx)
    ctx:prompt_text("Schema:", "schema", { default = "public" })
  end)
end

context:page("Review", function(ctx) end)

directory.render("contents", context)
]])
	ws:write("wizard/contents/out.toml", table.concat({
		'name = "{{ service_name }}"',
		'engine = "{{ engine }}"',
		"",
	}, "\n"))

	return ws
end)

--- One round of the loop: ask the archetype what is left, given what we know.
local function derive(t, ws, answers)
	local cmd = { t:use(bin), "interface", ws:file("wizard"), "--json" }
	for key, value in pairs(answers) do
		table.insert(cmd, "-a")
		table.insert(cmd, key .. "=" .. tostring(value))
	end
	return json.decode(shell.run(cmd, { check = true }).stdout)
end

--- The first page still holding something to ask. An EMPTY page is a finished
--- one — its prompts dropped out as they were answered — so a client that
--- rendered "the pages in the layout" without this check would show blank steps
--- for everything already done.
local function next_unfinished(layout)
	for _, node in ipairs(layout or {}) do
		if node.type == "page" and #(node.children or {}) > 0 then
			return node
		end
	end
	return nil
end

--- Stand in for a user filling the step in. Selects take the LAST option rather
--- than the default, so the loop actually walks into the conditional branches
--- instead of proving the easy path.
local function answer_for(prompt)
	if prompt.options and #prompt.options > 0 then
		return prompt.options[#prompt.options].value
	elseif prompt.type == "int" then
		return 25
	end
	return "loop-" .. prompt.key
end

prova.test("the hybrid loop terminates, and walks both shapes of conditional", {
	proves = "this is the algorithm a wizard implements, and the only thing that makes it "
		.. "safe to implement is that it TERMINATES. It does, because every round either "
		.. "answers something — shrinking what is left — or finds nothing left to ask. The "
		.. "loop needs no session, no server state, and no `appears_when` evaluator; each "
		.. "round is one idempotent query.",
}, function(t)
	local ws = t:use(ws_fixture)
	local answers, visited = {}, {}

	for round = 1, 12 do
		local iface = derive(t, ws, answers)
		local page = next_unfinished(iface.layout)
		if not page then
			break
		end
		visited[#visited + 1] = page.key
		for _, child in ipairs(page.children) do
			if child.type == "prompt" then
				for _, prompt in ipairs(iface.prompts) do
					if prompt.key == child.key then
						answers[child.key] = answer_for(prompt)
					end
				end
			end
		end
		t:expect(round < 12, "the loop terminates rather than spinning"):equals(true)
	end

	-- Identity, then Storage twice — the second visit is `pool`, which did not
	-- exist until `engine` was answered inside that same page — then the page
	-- that only exists down the postgres branch.
	t:expect(table.concat(visited, " → ")):equals("identity → storage → storage → postgres")

	-- What the loop collected is exactly what the render needs. No `-D`: an
	-- unanswered prompt would have to be asked, and there is no terminal to ask on.
	local cmd = { t:use(bin), "render", ws:file("wizard"), "--destination", ws:file("out") }
	for key, value in pairs(answers) do
		table.insert(cmd, "-a")
		table.insert(cmd, key .. "=" .. tostring(value))
	end
	local render = shell.run(cmd, { timeout = "60s" })
	t:expect(render.code, "the accumulated answers satisfy the whole render"):equals(0)
	t:expect(ws:read("out/out.toml")):contains('engine = "postgres"')
end)

prova.test("a finished page reports itself finished, which is what ends the loop", {
	proves = "the loop's termination check is 'no page has anything left', so a page whose "
		.. "prompts were all answered must come back EMPTY rather than come back with them "
		.. "listed. That is also the trap for a naive client: render the layout's pages "
		.. "without checking, and every completed step is a blank screen.",
}, function(t)
	local ws = t:use(ws_fixture)
	local before = derive(t, ws, {})
	local identity
	for _, node in ipairs(before.layout) do
		if node.key == "identity" then identity = node end
	end
	t:expect(#identity.children, "unanswered: it has something to ask"):equals(1)

	local after = derive(t, ws, { service_name = "billing" })
	for _, node in ipairs(after.layout) do
		if node.key == "identity" then identity = node end
	end
	t:expect(identity ~= nil, "the page is still in the layout"):equals(true)
	t:expect(#identity.children, "answered: nothing left to ask"):equals(0)
end)

prova.test("the step list is knowable in advance, in declaration order", {
	proves = "the loop alone cannot say 'step 2 of 4' — a page hidden behind an unselected "
		.. "branch is not in the layout yet, so the count grows as the user walks. One "
		.. "`--explore` call up front gives the superset for a progress indicator, and the "
		.. "client still never has to evaluate an `appears_when`: it uses exploration for "
		.. "the SHAPE and re-derivation for the CONTENT.",
}, function(t)
	local ws = t:use(ws_fixture)

	local bare = derive(t, ws, {})
	local visible = {}
	for _, node in ipairs(bare.layout) do
		if node.type == "page" then visible[#visible + 1] = node.key end
	end
	t:expect(table.concat(visible, ","), "the postgres step is not knowable yet")
		:equals("identity,storage,review")

	local explored = json.decode(shell.run(
		{ t:use(bin), "interface", ws:file("wizard"), "--json", "--explore" },
		{ check = true }
	).stdout)
	local all = {}
	for _, node in ipairs(explored.layout) do
		if node.type == "page" then all[#all + 1] = node.key end
	end
	t:expect(table.concat(all, ","), "the superset, in the order the author wrote it")
		:equals("identity,storage,postgres,review")
end)
