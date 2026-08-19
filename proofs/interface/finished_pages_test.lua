-- "Empty children means finished" — the rule the whole wizard loop turns on.
--
-- The hybrid drive is a loop of idempotent queries, and its termination check is
-- one line: *the first page in `layout` whose `children` are non-empty; none
-- means done*. That check is only sound if a page whose prompts have all been
-- answered actually comes back empty.
--
-- It did not. The prompts dropped out as they were answered, but the SECTIONS
-- they were declared inside stayed — so a page built from sections came back
-- holding a list of empty containers, read as "still has something to ask", and
-- a client implementing the documented algorithm looped forever. It went
-- unnoticed because the one client that drove it walked prompts recursively at
-- any depth instead of checking `children`, which is a workaround every client
-- would otherwise have to rediscover.
--
-- So containers are pruned by CONTENT: a section with nothing left in it is not
-- a fieldset, it is noise. Pages at the top of the layout are the exception and
-- survive empty, because that list is also the step list and a review step that
-- asks nothing is still a screen.
-- (docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty)

local workspace = require("prova.workspace")

local bin = require("subject").bin

local ws_fixture = prova.fixture("finished-pages-fixtures", Scope.File, function(ctx)
	local ws = workspace.create(ctx)

	-- Every shape that matters at once: prompts nested two sections deep, a page
	-- built ONLY of sections, a section the author declared with nothing in it,
	-- and a section that stays empty until a branch fills it.
	ws:write("wizard/archetype.yaml", 'description: "Wizard"\nrequires:\n  archetect: "3.0.0"\n')
	ws:write("wizard/archetype.lua", [[
local context = Context.new()

context:page({ title = "Service Identity", key = "identity" }, function(ctx)
  ctx:prompt_text("Service Name:", "service_name", { default = "orders" })
  ctx:section("Ownership", function(ctx)
    ctx:prompt_text("Team:", "team", { default = "platform" })
    ctx:section("Contact", function(ctx)
      ctx:prompt_text("Email:", "email", { default = "team@example.com" })
    end)
  end)
end)

context:page({ title = "Resources", key = "resources" }, function(ctx)
  ctx:section("Compute", function(ctx)
    ctx:prompt_int("CPU:", "cpu", { default = 1 })
  end)
  ctx:section("Storage", function(ctx)
    ctx:prompt_int("Disk:", "disk", { default = 10 })
  end)
end)

context:page({ title = "Database", key = "database" }, function(ctx)
  ctx:prompt_select("Engine:", "engine", { "none", "postgres" }, { default = "none" })
  ctx:section("Tuning", function(ctx)
    if ctx:get("engine") == "postgres" then
      ctx:prompt_text("Schema:", "schema", { default = "public" })
    end
  end)
end)

context:page("Review", function(ctx)
  ctx:section("Nothing To Confirm", function(ctx) end)
end)

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
local function derive(t, ws, answers, extra)
	local cmd = { t:use(bin), "interface", ws:file("wizard"), "--json" }
	for key, value in pairs(answers or {}) do
		table.insert(cmd, "-a")
		table.insert(cmd, key .. "=" .. tostring(value))
	end
	for _, arg in ipairs(extra or {}) do
		table.insert(cmd, arg)
	end
	return json.decode(shell.run(cmd, { check = true, timeout = "60s" }).stdout)
end

local function find_node(nodes, key)
	for _, node in ipairs(nodes or {}) do
		if node.key == key then
			return node
		end
		local found = find_node(node.children, key)
		if found then
			return found
		end
	end
	return nil
end

--- A flat, readable rendering of a subtree: `section:ownership>prompt:team`.
local function shape(nodes)
	local out = {}
	for _, node in ipairs(nodes or {}) do
		local label = node.type .. ":" .. node.key
		local children = shape(node.children)
		out[#out + 1] = children == "" and label or (label .. ">(" .. children .. ")")
	end
	return table.concat(out, ",")
end

-- ── the claim: a finished page is empty, sections and all ──────────

prova.test("a page whose sections are all answered comes back with nothing in it", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "the documented termination rule — 'first page with non-empty children; "
		.. "empty means finished' — is only sound if this holds. When the answered "
		.. "prompts vanished but their sections did not, every paginated archetype that "
		.. "used sections stayed non-empty forever and a client implementing the "
		.. "algorithm as written spun.",
}, function(t)
	local ws = t:use(ws_fixture)

	local before = derive(t, ws, {})
	t:expect(shape(find_node(before.layout, "identity").children), "unanswered: three fields deep")
		:equals("prompt:service_name,section:ownership>(prompt:team,section:contact>(prompt:email))")

	local after = derive(t, ws, {
		service_name = "billing", team = "platform", email = "team@example.com",
	})
	local identity = find_node(after.layout, "identity")
	t:expect(identity ~= nil, "the page is still in the layout — it is still a step"):equals(true)
	t:expect(#identity.children, "answered: nothing left, at any depth"):equals(0)
end)

prova.test("a page built only of sections empties out the same way", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "the reported shape: a page with no loose prompts at all, only sections. "
		.. "Measured on java-rest-service-archetype, `resources` came back holding four "
		.. "empty sections with every prompt answered — indistinguishable, to the "
		.. "termination check, from a page with four fields waiting.",
}, function(t)
	local ws = t:use(ws_fixture)
	local after = derive(t, ws, { cpu = 4, disk = 100 })
	t:expect(#find_node(after.layout, "resources").children):equals(0)
end)

prova.test("a section still holding a prompt survives, with only what is left", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "the negative control, and the one that makes the rule usable rather than "
		.. "merely terminating: pruning is by CONTENT, so a partly-answered page keeps "
		.. "exactly the structure around what it still needs. Without this, a rule that "
		.. "emptied pages on a timer or on first answer would pass every proof above and "
		.. "lose fields.",
}, function(t)
	local ws = t:use(ws_fixture)
	local after = derive(t, ws, { service_name = "billing", team = "platform" })
	t:expect(shape(find_node(after.layout, "identity").children), "the path down to `email` is intact")
		:equals("section:ownership>(section:contact>(prompt:email))")
end)

prova.test("a section the author declared with nothing in it never reaches the client", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "a section is a fieldset within a step; a fieldset with no fields has "
		.. "nothing to render and no identity a client routes on. Emitting it would put "
		.. "the loop back where it started for any author who declared one.",
}, function(t)
	local ws = t:use(ws_fixture)
	local review = find_node(derive(t, ws, {}).layout, "review")
	t:expect(review ~= nil, "its page is still there"):equals(true)
	t:expect(#review.children, "the empty section is not"):equals(0)
	t:expect(find_node(derive(t, ws, {}).layout, "nothing_to_confirm")):equals(nil)
end)

prova.test("an empty page is still a step, and still in the step list", {
	proves = "the boundary of the rule. A page is a screen the user walks through and "
		.. "the top-level layout is the step list — dropping a review step that asks "
		.. "nothing would lose a screen, which is the opposite failure and is exactly "
		.. "why sections and pages are not treated alike.",
}, function(t)
	local ws = t:use(ws_fixture)
	local steps = {}
	for _, node in ipairs(derive(t, ws, {}).layout) do
		if node.type == "page" then
			steps[#steps + 1] = node.key
		end
	end
	t:expect(table.concat(steps, ",")):equals("identity,resources,database,review")
end)

-- ── the loop the rule exists for ───────────────────────────────────

prova.test("the documented loop terminates on an archetype built from sections", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "the algorithm transcribed from the plan, with no recursive workaround in "
		.. "its termination check — because that check is the contract, and a client that "
		.. "has to discover 'actually, walk prompts at any depth' has been handed a rule "
		.. "that does not work. Collecting a page's fields is still recursive; laying out "
		.. "a nested form always is. Deciding whether the page is DONE is not.",
}, function(t)
	local ws = t:use(ws_fixture)
	local answers, visited = {}, {}

	--- Straight from the plan: the first page with non-empty children.
	local function next_unfinished(layout)
		for _, node in ipairs(layout or {}) do
			if node.type == "page" and #(node.children or {}) > 0 then
				return node
			end
		end
		return nil
	end

	local function collect(nodes, iface)
		for _, node in ipairs(nodes or {}) do
			if node.type == "prompt" then
				for _, prompt in ipairs(iface.prompts) do
					if prompt.key == node.key then
						if prompt.options and #prompt.options > 0 then
							answers[node.key] = prompt.options[#prompt.options].value
						elseif prompt.type == "int" then
							answers[node.key] = 7
						else
							answers[node.key] = "loop-" .. node.key
						end
					end
				end
			else
				collect(node.children, iface)
			end
		end
	end

	local rounds = 0
	while true do
		rounds = rounds + 1
		t:expect(rounds < 12, "the loop terminates rather than spinning"):equals(true)
		if rounds >= 12 then
			break
		end
		local iface = derive(t, ws, answers)
		local page = next_unfinished(iface.layout)
		if not page then
			break
		end
		visited[#visited + 1] = page.key
		collect(page.children, iface)
	end

	-- Database twice: `Tuning` is empty until `engine` is answered inside that
	-- same page, and comes back holding `schema` on the next round. Review is
	-- never visited — it has nothing to ask, so the loop is right to skip it.
	t:expect(table.concat(visited, " → "))
		:equals("identity → resources → database → database")

	local cmd = { t:use(bin), "render", ws:file("wizard"), "--destination", ws:file("out") }
	for key, value in pairs(answers) do
		table.insert(cmd, "-a")
		table.insert(cmd, key .. "=" .. tostring(value))
	end
	local render = shell.run(cmd, { timeout = "60s" })
	t:expect(render.code, "what the loop collected is what the render needs"):equals(0)
	t:expect(ws:read("out/out.toml")):contains('engine = "postgres"')
end)

-- ── the superset is still a superset ───────────────────────────────

prova.test("--explore still maps a section that only fills in down one branch", {
	covers = "docs/plans/interface-pages-and-sections.md#finished-pages-are-not-empty",
	proves = "pruning must run on the MERGED tree, not per run, or exploration would "
		.. "throw away exactly what it is for. `Tuning` is empty on the default path and "
		.. "holds `schema` down the postgres branch; a client asking for the shape up "
		.. "front must get the union, while the default-path derivation stays honest "
		.. "about there being nothing there yet.",
}, function(t)
	local ws = t:use(ws_fixture)

	local plain = find_node(derive(t, ws, {}).layout, "database")
	t:expect(shape(plain.children), "default path: the section holds nothing, so it is not there")
		:equals("prompt:engine")

	local explored = find_node(derive(t, ws, {}, { "--explore" }).layout, "database")
	t:expect(shape(explored.children), "explored: the branch fills it in, so it survives")
		:equals("prompt:engine,section:tuning>(prompt:schema)")
end)
