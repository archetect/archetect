--- The deputed unit-test leg: `prova run ut` conducts `cargo nextest` ONCE and
--- adopts every case into the account.
---
--- Prova and cargo test answer different questions and this file is the seam,
--- not a replacement: prova proves what archetect SHIPS by driving the binary;
--- the unit tests prove the pieces the binary is made of. Before this, the two
--- accounts were separate — `prova` could be green while `cargo test` was red,
--- and nothing said so in one place.
---
--- HEAVY: conducting compiles the workspace. Behind the `ut` switch (suite.lua),
--- with `cargo-nextest` as a `requires` world fact — intent and world are two
--- facts with two remedies. The `ut` profile `must_run`s the deputy, so
--- `prova run ut` FAILS rather than skips when nextest is missing: a profile is
--- a contract, not a courtesy.

-- Scope.Run: one conduct per run, whatever reads it. The exit code is
-- deliberately not asserted at conduct time — the adopting proof below reports
-- red with the deputed cases' OWN names, which a dead fixture would hide.
local custody = require("custody")

local nextest = prova.fixture("nextest-junit", Scope.Run, function()
	local artifact = prova.root .. "/target/nextest/prova/junit.xml"
	-- Removed FIRST, so a deputy that dies before emitting leaves nothing behind
	-- and the adoption fails loudly on "matched nothing" — never a previous
	-- run's verdicts wearing this run's face.
	fs.remove_all(artifact)

	local cmd = { "cargo", "nextest", "run", "--workspace", "--profile", "prova" }
	-- Selection pushdown: the run's `-k` keywords narrow the conduct to matching
	-- case names, so one vocabulary works at both granularities. Only clean
	-- identifier-shaped keywords ride — anything else could bend nextest's
	-- expression grammar, and conducting FULL is the safe over-approximation.
	for _, keyword in ipairs(prova.selection.keywords) do
		if keyword:match("^[%w_:-]+$") then
			cmd[#cmd + 1] = "-E"
			cmd[#cmd + 1] = "test(" .. keyword .. ")"
		end
	end
	shell.run(cmd, {
		cwd = prova.root,
		merge_stderr = true,
		-- Priced for the longest SILENT stretch, not a heartbeat: cargo is chatty
		-- while tests run, but a single big crate's codegen says nothing for
		-- minutes, and archetect-core links a vendored Lua and protoc output.
		idle_timeout = "600s",
		timeout = "1800s",
	})

	-- Custody: the account adopts every CASE, but the artifact itself lives under
	-- `target/`, which gets swept — taking the detail behind a red case (stdout,
	-- the failure message, timings) with it. Publishing keeps it addressable as
	-- `prova reports unit-cases`.
	if fs.exists(artifact) then
		local adopted = junit.load(artifact)
		custody.publish {
			name = "unit-cases",
			summary = string.format(
				"%d cases · %d passed · %d failed · %d skipped",
				adopted.total,
				adopted.passed,
				adopted.failed + adopted.errors,
				adopted.skipped
			),
			forms = { xml = artifact },
		}
	end

	return artifact
end)

prova.test("the workspace's unit-test account holds — every nextest case adopted", {
	-- Cargo takes process-wide locks of its own, so two prova instances that both
	-- reach for it contend unpredictably unless the house rule is said out loud.
	locks = { prova.writes("cargo") },
	requires = { "cargo-nextest" },
	proves = "one `prova` is the whole quality account or it is a partial one. The unit "
		.. "tests were a second, separate green that nothing in this suite could speak "
		.. "for — adopting their verdicts is what makes a single exit code honest.",
}, function(t)
	junit.verify(t, { results = t:use(nextest) })
end)

-- Readers: one claim, one named unit test. This is the granularity the pattern
-- exists for — a specific case discharging a specific contract, at the cost of a
-- parse rather than a second compile.
local function case(report_, name)
	for _, c in ipairs(report_.cases) do
		if c.name == name or c.name:match(name .. "$") then
			return c
		end
	end
end

prova.test("segment key derivation is pinned at the unit level, beside its black-box proof", {
	locks = { prova.writes("cargo") },
	requires = { "cargo-nextest" },
	proves = "`proofs/interface/pages_and_sections_test.lua` proves the slug through the "
		.. "shipped binary on the cases an archetype actually uses. The edges — a title of "
		.. "pure punctuation, an empty one — are cheaper to pin in a unit test than to "
		.. "author an archetype for, and this binds that unit test to the same account.",
}, function(t)
	local adopted = junit.load(t:use(nextest))
	for _, name in ipairs({
		"derives_a_snake_slug_from_a_title",
		"an_unsluggable_title_still_yields_a_key",
	}) do
		local c = case(adopted, name)
		t:expect(c, name .. " exists in the deputed account"):is_truthy()
		t:expect(c and c.outcome, name):equals("passed")
	end
end)
