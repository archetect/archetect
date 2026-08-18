--- Quality gate: shipped code does not sprout new panics.
---
--- `.unwrap()` and `.expect()` are latent panics. Test code uses them freely and
--- idiomatically, so only lib and bin targets are counted — clippy's restriction
--- lints exclude tests by construction. The counts are ratcheted, not zeroed:
--- the ones already here are grandfathered, new ones are red, and removing them
--- is welcome (`prova run quality --update-baseline` tightens the floor).
---
--- HEAVY: recompiles with the restriction lints enabled — a different lint set
--- is a different fingerprint, so this does not reuse the plain clippy gate's
--- artifacts. One conduct feeds both ratchets.

-- `--keep-going` and the span filter, for the reason clippy_test.lua spells out:
-- archetect-inflections' upstream `#![deny(warnings)]` escalates these very
-- lints to hard errors, aborting its compilation and taking the rest of the
-- workspace's diagnostics with it. Measured live: 4 and 7 for identical code.
local UNGATED = "archetect-inflections/"

-- Generated code is not this repo's code. `archetect-core/build.rs` compiles the
-- proto into `target/.../out/archetect.rs`, and prost's output trips lints nobody
-- here can fix — it would put a dependency's codegen style into archetect's debt
-- number, and change it on a prost bump with no commit to explain the move.
local GENERATED = "/target/"

local restricted = prova.fixture("clippy-restrictions", Scope.File, function()
	local out = shell.run({
		"cargo", "clippy", "--workspace", "--lib", "--bins", "--keep-going",
		"--message-format=json",
		"--", "-W", "clippy::unwrap_used", "-W", "clippy::expect_used",
	}, { cwd = prova.root, timeout = "1800s", idle_timeout = "600s" })

	local counts, sites = {}, {}
	for line in (out.stdout or ""):gmatch("[^\n]+") do
		local ok, msg = pcall(json.decode, line)
		if ok and type(msg) == "table" and msg.reason == "compiler-message" then
			local m = msg.message or {}
			local code = m.code and m.code.code
			local span = m.spans and m.spans[1]
			local file = (span and span.file_name) or "?"
			if (code == "clippy::unwrap_used" or code == "clippy::expect_used")
				and not file:find(UNGATED, 1, true)
				and not file:find(GENERATED, 1, true) then
				counts[code] = (counts[code] or 0) + 1
				sites[#sites + 1] = string.format("%s  %s:%d",
					code:gsub("clippy::", ""), file, span and span.line_start or 0)
			end
		end
	end
	table.sort(sites)
	return { counts = counts, sites = sites }
end)

--- Name every site before ratcheting: a failing assertion aborts the body, so a
--- worklist printed after it is missing on exactly the runs that need it.
local function gate(t, code, id)
	local measured = t:use(restricted)
	local prefix = code:gsub("clippy::", "")
	for _, site in ipairs(measured.sites) do
		if site:sub(1, #prefix) == prefix then
			print("  " .. site)
		end
	end
	measure.ratchet(t, id, measured.counts[code] or 0, { set = "quality" })
end

prova.test("shipped .unwrap() calls do not multiply past the baseline", {
	switch = "quality",
	locks = { prova.writes("cargo") },
	proves = "a panic in a code generator loses whatever the user was halfway through "
		.. "generating, with a backtrace instead of a message. The ratchet does not demand "
		.. "the existing ones be paid off today; it demands the number stop growing.",
}, function(t)
	gate(t, "clippy::unwrap_used", "rust.unwrap.shipped")
end)

prova.test("shipped .expect() calls do not multiply past the baseline", {
	switch = "quality",
	locks = { prova.writes("cargo") },
	proves = "an `.expect(\"...\")` is a better panic than an `.unwrap()`, not a substitute "
		.. "for an error — counting it separately keeps the distinction visible instead of "
		.. "letting it become the way around the unwrap gate.",
}, function(t)
	gate(t, "clippy::expect_used", "rust.expect.shipped")
end)
