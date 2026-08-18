--- Quality gate: clippy findings do not multiply.
---
--- The honest posture for a tree with standing debt. `-D warnings` on a
--- workspace carrying ~146 findings is a gate that is red on arrival, which
--- teaches everyone to run with it off; a RATCHET is red only when the number
--- goes UP, so the bar can only move one way without a deliberate, reviewable
--- edit to a committed baseline. Paid some down? `prova run quality
--- --update-baseline` tightens the floor (it refuses to loosen).
---
--- HEAVY: clippy recompiles the workspace. Behind the `quality` switch — off
--- unless thrown. `prova run quality` throws it; `prova -s quality` is the
--- ad-hoc door.

-- WHY the count is measured this way, and why it is not a loophole.
--
-- `archetect-inflections` carries `#![deny(warnings)]` from its upstream (it is
-- a fork of the Inflector crate), which turns every clippy lint in it into a
-- hard ERROR and ABORTS its compilation. Without `--keep-going`, downstream
-- crates then never get linted at all, and the total depends on how far the
-- build got before the abort: measured live, three consecutive whole-workspace
-- counts read 221, 243, and 221 with no code change between them. A ratchet on
-- a number that moves by twenty on its own is worse than no ratchet — it fails
-- randomly and gets switched off.
--
-- So: `--keep-going` lints everything regardless, and that crate's own findings
-- are filtered out by SPAN rather than by excluding it from the build. Filtering
-- after the fact is what keeps its debt honest — it stays measurable (the
-- promise at the bottom of this file counts it), it just does not corrupt the
-- rest. Stable to the unit this way: 146, 146, 146.
local UNGATED = "archetect-inflections/"

-- Generated code is not this repo's code. `archetect-core/build.rs` compiles the
-- proto with prost, whose output trips lints nobody here can fix — counting them
-- would put a dependency's codegen style into archetect's debt number and move it
-- on a prost bump with no commit to explain the change.
--
-- Identified by being OUTSIDE the workspace, not by a path substring. Cargo
-- reports workspace sources as repo-relative (`archetect-core/src/...`) and
-- everything else absolute, so a leading slash IS the test. The substring this
-- replaces (`/target/`) was measured against a build that happened to put OUT_DIR
-- under the repo; cargo's build-dir is a machine-level setting, and when it moved
-- to `~/.cache/cargo/build/...` the filter silently matched nothing and the count
-- jumped 138 → 146 with no code change. A ratchet that depends on where cargo
-- keeps its scratch files is not measuring the codebase.
--
-- (Unix-shaped: a Windows absolute path starts `C:\`. The quality lane runs on
-- macOS and Linux; revisit if that changes.)
local function is_ours(file)
	return file:sub(1, 1) ~= "/" and not file:find(UNGATED, 1, true)
end

local custody = require("custody")

local findings = prova.fixture("clippy-findings", Scope.File, function()
	local out = shell.run({
		"cargo", "clippy", "--workspace", "--all-targets", "--keep-going",
		"--message-format=json",
	}, { cwd = prova.root, timeout = "1800s", idle_timeout = "600s" })

	-- Count DIAGNOSTICS, not lines: cargo's human output interleaves the same
	-- finding with its help/note/suggestion block, so a line count moves when
	-- clippy reworks a suggestion. A coded compiler-message is one finding.
	local by_lint, total = {}, 0
	local sample = {}
	for line in (out.stdout or ""):gmatch("[^\n]+") do
		local ok, msg = pcall(json.decode, line)
		if ok and type(msg) == "table" and msg.reason == "compiler-message" then
			local m = msg.message or {}
			local code = m.code and m.code.code
			local file = (m.spans and m.spans[1] and m.spans[1].file_name) or "?"
			if code and (m.level == "warning" or m.level == "error") and is_ours(file) then
				total = total + 1
				by_lint[code] = (by_lint[code] or 0) + 1
				sample[code] = sample[code] or file
			end
		end
	end

	local path = prova.root .. "/target/clippy-findings.json"
	fs.write(path, json.encode({ total = total, by_lint = by_lint, sample = sample }))
	custody.publish {
		name = "clippy",
		summary = string.format("%d findings across %d lints (%s not gated yet)",
			total, (function() local n = 0 for _ in pairs(by_lint) do n = n + 1 end return n end)(), UNGATED),
		explains = "rust.clippy.findings",
		forms = { json = path },
	}
	return { total = total, by_lint = by_lint, sample = sample, ran = out.code ~= nil }
end)

prova.test("clippy findings do not multiply past the baseline", {
	switch = "quality",
	-- Cargo takes process-wide locks of its own; two prova instances that both
	-- reach for it contend unpredictably unless the house rule is said out loud.
	locks = { prova.writes("cargo") },
	proves = "green proofs are not the same as ready to land. A proof suite says the "
		.. "behavior is right; it says nothing about a lint added on the way there. This "
		.. "is where that difference gets caught, by a number that can only go down.",
}, function(t)
	-- Say which clippy answered. A gate is only as good as the tool behind it, and
	-- the classic failure is passing locally while failing in CI because the two
	-- ran different versions.
	local version = shell.run({ "cargo", "clippy", "--version" },
		{ cwd = prova.root, merge_stderr = true })
	t:log("clippy: " .. (version.stdout or ""):gsub("%s+$", ""))

	local measured = t:use(findings)

	-- Vacuity guard: a broken invocation reads as zero findings, which a
	-- lower-is-better ratchet would happily accept as a triumph.
	t:expect(measured.total > 0, "clippy produced findings to count — a zero here means "
		.. "the invocation broke, not that the tree is clean"):equals(true)

	-- The worklist, printed BEFORE the ratchet: a failing assertion aborts the
	-- body, so anything printed after it is missing on exactly the runs that need it.
	local rows = {}
	for lint, count in pairs(measured.by_lint) do
		rows[#rows + 1] = { lint = lint, count = count }
	end
	table.sort(rows, function(a, b) return a.count > b.count end)
	for i = 1, math.min(#rows, 10) do
		print(string.format("  %-48s %4d  e.g. %s", rows[i].lint, rows[i].count,
			measured.sample[rows[i].lint]))
	end

	measure.ratchet(t, "rust.clippy.findings", measured.total, { set = "quality" })
end)

prova.test("archetect-inflections lints under the same gate as the rest of the workspace", {
	switch = "quality",
	locks = { prova.writes("cargo") },
	promises = "the crate's upstream `#![deny(warnings)]` turns clippy lints into hard "
		.. "errors and aborts its compilation, so the gate above filters its findings out "
		.. "by span and its debt is counted by nobody. Fixing it means clearing its own "
		.. "findings (mostly `assert_eq!(x, true)` in its tests) or scoping the deny — a "
		.. "self-contained job in a forked crate rather than part of any feature.",
}, function(t)
	local out = shell.run({
		"cargo", "clippy", "-p", "archetect-inflections", "--all-targets", "--", "-D", "warnings",
	}, { cwd = prova.root, timeout = "900s", merge_stderr = true })
	t:expect(out.code, "clippy is clean for archetect-inflections:\n" .. (out.stdout or "")):equals(0)
end)
