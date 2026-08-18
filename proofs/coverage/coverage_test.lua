--- Coverage, layered: ONE conduct, three numbers.
---
--- A unit-only coverage figure would lie about this repo specifically. Archetect
--- is proven black-box — the suite drives the shipped CLI, and whole subsystems
--- (the server, the connect client, the MCP surface, the interface probe) are
--- reached almost entirely that way. Measuring only `cargo test` would report
--- them as untested and send someone off to write unit tests for behavior that
--- is already proven, while saying nothing about the code no layer touches.
---
--- So the conduct measures both and reports each ALONE and MERGED:
---
---   unit       `cargo llvm-cov nextest --workspace`
---   black-box  the whole prova suite, driving an INSTRUMENTED archetect
---   merged     both profraw sets, one whole-bar total
---
--- The DELTA is the signal. A file rich in black-box coverage but naked at the
--- unit layer is proven behavior with no fast local feedback — the place
--- granular unit tests still earn their keep. The worklist at the bottom
--- computes exactly that, rather than leaving it to taste.
---
--- Layout: everything is pinned into `target/llvm-cov-target` via
--- `CARGO_LLVM_COV_TARGET_DIR`, and profraws land at that directory's root,
--- which is also where `report` scans. Nothing instrumented is allowed near
--- `target/debug` — an instrumented archetect there would be the binary every
--- later `prova` run drives, silently dropping a profraw wherever it was invoked.

local COV_DIR = prova.root .. "/target/llvm-cov-target"
local UNIT_STAGE = prova.root .. "/target/coverage-unit-stage"

-- WHERE THE FLOORS SIT, and why they are not the high-water mark.
--
-- A ratchet banked at a peak fails forever after. Each floor here is banked a
-- band below the measurement that established it, and the band is sized to the
-- layer's own volatility:
--
--   unit       1.0  ordinary feature work dilutes it before anyone gets to the
--                   tests; a zero-tolerance floor gates commits on that lag.
--   blackbox   1.0  its denominator moves when instrumented objects enter or
--                   leave the scan, which is not a coverage change at all.
--   merged     0.5  the union moves least — a proof-first tree covers new
--                   behavior black-box as it lands.
--
-- The band is not slack to spend. It is the distance at which a trip still has
-- obvious material behind it: trip a floor a point down and the unit-owed
-- worklist below names files with 40+ point deltas — real work, chosen by
-- consequence. Trip it at 0.01pp and the only moves left are the ones that
-- damage the codebase: tests that execute lines without asserting anything.
--
-- Raising a floor is `prova run coverage --update-baseline` (it tightens only).
-- LOWERING one is a hand edit of .prova/baselines/quality.json, deliberately,
-- with the reason in the commit — never as a way past a red gate.

--- The env `cargo llvm-cov` subcommands need: just where to put everything.
---
--- Deliberately NOT the full show-env map. That map carries `RUSTC_WRAPPER` and
--- the wrapper's private `__CARGO_LLVM_COV_*` variables, which exist so a PLAIN
--- `cargo build` can be instrumented. Handing them back to `cargo llvm-cov`
--- itself double-wraps: its `cargo metadata` probe (`rustc - --crate-name ___
--- --print=file-names`) goes through the wrapper and exits 1, and the whole
--- nextest leg dies with a wall of identical rustc errors.
local function llvm_cov_env()
	return { CARGO_LLVM_COV_TARGET_DIR = COV_DIR }
end

--- `cargo llvm-cov show-env` as a table (values arrive single-quoted). This IS
--- the full map, and it is for the plain `cargo build` of the subject — the one
--- invocation that has no llvm-cov wrapper of its own.
local function build_env()
	local out = shell.run({ "cargo", "llvm-cov", "show-env" },
		{ cwd = prova.root, env = llvm_cov_env() })
	local env = {}
	for line in (out.stdout or ""):gmatch("[^\n]+") do
		local key, value = line:match("^([%w_]+)='?([^']*)'?$")
		if key then
			env[key] = value
		end
	end
	-- show-env reports its own default for this one; the whole point of the
	-- conduct is that it is somewhere else.
	env.CARGO_LLVM_COV_TARGET_DIR = COV_DIR
	return env
end

local function purge(dir, pattern)
	for _, path in ipairs(fs.glob(dir, pattern)) do
		fs.remove_all(path)
	end
end

--- Report over whatever profraws are at the scan root right now.
---
--- The cached profdata is purged FIRST. `report` reuses it when it is there and
--- silently ignores every profraw written since the last merge — which reads as
--- "three layers with identical totals" rather than as an error.
local function fresh_report()
	purge(COV_DIR, "*.profdata")
	local out = shell.run({ "cargo", "llvm-cov", "report", "--json" },
		{ cwd = prova.root, env = llvm_cov_env(), timeout = "600s" })
	return json.decode(out.stdout or "{}")
end

local function totals(report_)
	return report_.data[1].totals.lines
end

local function pct(report_)
	return totals(report_).percent
end

--- Move every profraw between the scan root and the stage. Loud when a staging
--- that must move files moves none: a silent no-op is how a "merge" ends up
--- reporting the same set twice.
local function stage(into_stage)
	fs.mkdir(UNIT_STAGE)
	local from = into_stage and COV_DIR or UNIT_STAGE
	local to = into_stage and UNIT_STAGE or COV_DIR
	local moved = 0
	for _, path in ipairs(fs.glob(from, "*.profraw")) do
		shell.run({ "mv", path, to .. "/" }, { cwd = prova.root })
		moved = moved + 1
	end
	return moved
end

local conduct = prova.fixture("layered-coverage", Scope.File, function()
	-- Data-only clean: the instrumented build artifacts are the expensive stage
	-- and stay for incremental conducts. Stale DATA, though, is a previous run's
	-- verdicts wearing this one's face.
	fs.mkdir(COV_DIR)
	purge(COV_DIR, "*.profraw")
	purge(COV_DIR, "*.profdata")
	-- llvm-cov's own index of which profraws it last merged. Left behind, it
	-- makes `report` answer for a previous conduct's data set.
	purge(COV_DIR, "*-profraw-list")
	fs.remove_all(UNIT_STAGE)

	-- A stale-GENERATION guard. Instrumented objects from a different toolchain
	-- inflate the denominator, and a target dir carrying a half-migrated llvm-cov
	-- layout breaks the next conduct in a way that reads as a test failure
	-- (observed: `cargo llvm-cov nextest` failing its own rustc probe with a wall
	-- of identical errors). Cheap to detect, expensive to debug.
	local stamp_path = COV_DIR .. "/.archetect-coverage-stamp"
	local rustc = shell.run({ "rustc", "--version" }, { cwd = prova.root })
	local stamp = (rustc.stdout or "?"):gsub("%s+$", "")
	if not fs.exists(stamp_path) or fs.read(stamp_path) ~= stamp then
		fs.remove_all(COV_DIR)
		fs.mkdir(COV_DIR)
		fs.write(stamp_path, stamp)
	end

	-- ORDER MATTERS, and not for the reason you would guess.
	--
	-- The unit layer runs FIRST because `report` derives its denominator from
	-- every instrumented object it can scan, and nextest's test binaries are part
	-- of that set. Report the black-box layer before they exist and it answers
	-- over 14,946 lines; report it after and the same data reads over 20,695 —
	-- a ten-point "regression" between a cold conduct and a warm one, with no
	-- code change anywhere.
	--
	-- Two ways out: hide the test binaries while the black-box layer reports
	-- (a smaller, arguably purer denominator), or make sure they are always there
	-- (one denominator for all three layers). This takes the second, because the
	-- point of measuring three layers is COMPARING them: unit 61% and black-box
	-- 27% only add up to a merged 72% if all three count the same lines, and the
	-- unit-owed worklist at the bottom subtracts one layer's per-file percent from
	-- the other's.
	--
	-- The cache llvm-cov's rustc probe keeps here is dropped first: a plain
	-- instrumented `cargo build` (below) writes it in a shape `cargo llvm-cov`
	-- then chokes on. It is a probe cache — rebuilding it costs one rustc call.
	fs.remove_all(COV_DIR .. "/.rustc_info.json")
	local unit_run = shell.run({ "cargo", "llvm-cov", "nextest", "--workspace", "--no-report" },
		{ cwd = prova.root, env = llvm_cov_env(), timeout = "1800s", idle_timeout = "600s",
		  merge_stderr = true })
	if unit_run.code ~= 0 then
		return { error = "the unit tests are red under instrumentation:\n"
			.. (unit_run.stdout or ""):sub(-4000) }
	end
	local unit = fresh_report()
	if stage(true) == 0 then
		return { error = "staging moved no unit profraws — the scan-root assumption broke" }
	end

	-- The instrumented subject. `--target-dir` is explicit: without it this lands
	-- in target/debug and every later `prova` invocation silently drives an
	-- instrumented archetect, littering profraws wherever it is run from.
	local build = shell.run({ "cargo", "build", "-p", "archetect", "--target-dir", COV_DIR },
		{ cwd = prova.root, env = build_env(), timeout = "1800s", merge_stderr = true })
	if build.code ~= 0 then
		return { error = "instrumented build failed:\n" .. (build.stdout or "") }
	end
	local subject = COV_DIR .. "/debug/archetect"

	-- The black-box layer: the whole suite, through the instrumented binary.
	--
	-- The subject reaches the inner run as an env var rather than a switch,
	-- because the inner run is a DIFFERENT prova process
	-- (.prova/packages/subject/init.lua honors it and skips its own build).
	-- The inner run throws no switches, so it does not recurse into this lane.
	local suite = shell.run({ prova.bin }, {
		cwd = prova.root,
		timeout = "1800s",
		idle_timeout = "600s",
		merge_stderr = true,
		env = {
			ARCHETECT_SUBJECT_BIN = subject,
			LLVM_PROFILE_FILE = COV_DIR .. "/suite-%p-%28m.profraw",
		},
	})
	if suite.code ~= 0 then
		-- The tail of the capture is not the failure: a run's summary is the last
		-- thing printed, but it is a count, not a diagnosis. Select the reporter's
		-- own verdict lines and keep the whole capture beside them.
		local log = prova.root .. "/target/coverage-suite.log"
		fs.write(log, suite.stdout or "")
		local verdicts = {}
		for line in (suite.stdout or ""):gmatch("[^\n]+") do
			if line:match("FAIL") or line:match("%d+ passed,") then
				verdicts[#verdicts + 1] = line
			end
		end
		if #verdicts == 0 then
			verdicts[1] = "(the suite printed no verdict line — it died rather than reported)"
		end
		return {
			error = "the black-box suite is red under instrumentation — fix the bar before "
				.. "measuring it:\n" .. table.concat(verdicts, "\n") .. "\n  full capture: " .. log,
		}
	end

	-- Did the SUBJECT get measured? This layer's number is entirely what the
	-- archetect processes the suite spawned executed. If the env var stopped
	-- reaching them, the suite would still pass, the report would still produce a
	-- number, and the only symptom would be a large unexplained "regression".
	-- One profraw per instrumented process, so the floor is a did-it-happen-at-all
	-- guard set far below a healthy conduct (dozens) — not a count to tune.
	local emitted = #fs.glob(COV_DIR, "suite-*.profraw")
	if emitted < 10 then
		return {
			error = string.format(
				"the black-box layer measured almost nothing: %d suite profraw(s). The subject "
					.. "is probably not the instrumented build — check that ARCHETECT_SUBJECT_BIN "
					.. "reaches the inner run and that %s is what its proofs drove.",
				emitted, subject),
		}
	end

	local blackbox = fresh_report()

	-- The merge: the unit profraws rejoin, one whole-bar total.
	stage(false)
	local merged = fresh_report()

	-- Custody. The conduct has produced the answer to "which lines"; without this
	-- it is discarded with `target/`, and a red floor can refuse a regression
	-- while being unable to show what moved. Two forms of one fact: llvm-cov's
	-- own HTML for a person, the merged JSON for an agent (and for diffing runs).
	local html_dir = prova.root .. "/target/coverage-html"
	fs.remove_all(html_dir)
	local html = shell.run(
		{ "cargo", "llvm-cov", "report", "--html", "--output-dir", html_dir },
		{ cwd = prova.root, env = llvm_cov_env(), timeout = "600s" })
	local json_path = prova.root .. "/target/coverage-merged.json"
	fs.write(json_path, json.encode(merged))

	local forms = { json = json_path }
	-- llvm-cov writes a TREE at <output-dir>/html — a page per file, linked from
	-- an index. The whole tree is published; the entry point alone would file a
	-- report with every link broken. If the HTML pass fails the JSON still goes:
	-- a missing human form is worth less than the whole report going away.
	if html.code == 0 and fs.exists(html_dir .. "/html/index.html") then
		forms.html = html_dir .. "/html"
	end

	report.publish {
		name = "coverage",
		summary = string.format("unit %.2f%% · black-box %.2f%% · merged %.2f%% (%d/%d lines)",
			pct(unit), pct(blackbox), pct(merged),
			totals(merged).covered, totals(merged).count),
		-- Named so a red ratchet can point at the evidence instead of leaving the
		-- reader to rebuild the conduct.
		explains = { "rust.coverage.unit", "rust.coverage.blackbox", "rust.coverage.lines" },
		forms = forms,
	}

	return { unit = unit, blackbox = blackbox, merged = merged }
end)

--- Per-file line percent from a full report.
local function by_file(report_)
	local out = {}
	for _, file in ipairs(report_.data[1].files or {}) do
		out[file.filename] = file.summary.lines.percent
	end
	return out
end

prova.test("whole-bar line coverage — unit AND black-box merged — does not regress", {
	locks = { prova.writes("cargo") },
	requires = { "cargo-llvm-cov", "cargo-nextest" },
	proves = "the merged total is the only number that describes this project honestly. "
		.. "Unit-only reads the server and the connect client as near-zero when they are "
		.. "among the most heavily proven code in the tree; black-box-only misses "
		.. "everything the CLI cannot reach. A figure that misleads at the edges is worse "
		.. "than none, because someone will act on it.",
}, function(t)
	local produced = t:use(conduct)
	t:expect(produced.error, produced.error or "the conduct produced reports"):is_nil()

	-- The merge is real, or the gate is measuring one layer three times. Identical
	-- totals mean `report` read cached profdata or a half-staged profraw set — the
	-- exact failure the profdata purge exists to prevent.
	t:expect(
		pct(produced.merged) == pct(produced.unit) and pct(produced.unit) == pct(produced.blackbox),
		"unit, black-box, and merged are identical — the reports are not seeing distinct "
			.. "profraw sets"
	):is_false()

	-- And the three are measured over the SAME code. If a layer's denominator
	-- moves, its percent moves with it and nothing in the codebase changed — the
	-- cheapest guard against attributing a scan artifact to lost coverage.
	t:expect(totals(produced.unit).count, "unit and merged measure the same denominator")
		:equals(totals(produced.merged).count)
	t:expect(totals(produced.blackbox).count, "black-box and merged measure the same denominator")
		:equals(totals(produced.merged).count)

	measure.ratchet(t, "rust.coverage.lines", pct(produced.merged), {
		set = "quality", direction = "higher_is_better",
	})
end)

prova.test("each layer holds on its own — and the delta names where unit tests are owed", {
	locks = { prova.writes("cargo") },
	requires = { "cargo-llvm-cov", "cargo-nextest" },
	proves = "the delta is the worklist. Proven-black-box but unit-naked files are "
		.. "behavior with no fast local feedback: correct today, and slow to iterate on "
		.. "tomorrow. Computing that list beats guessing which module deserves attention.",
}, function(t)
	local produced = t:use(conduct)
	t:expect(produced.error, produced.error or "the conduct produced reports"):is_nil()

	-- Denominators printed BEFORE any ratchet fires: a failing assertion aborts the
	-- body, so anything printed after it is missing on exactly the runs that need it.
	for _, layer in ipairs({
		{ "unit", produced.unit },
		{ "blackbox", produced.blackbox },
		{ "merged", produced.merged },
	}) do
		local line = totals(layer[2])
		print(string.format("  layer %-9s %6.2f%%  %6d/%-6d lines  %4d files",
			layer[1], line.percent, line.covered, line.count,
			#(layer[2].data[1].files or {})))
	end

	measure.ratchet(t, "rust.coverage.unit", pct(produced.unit), {
		set = "quality", direction = "higher_is_better",
	})
	measure.ratchet(t, "rust.coverage.blackbox", pct(produced.blackbox), {
		set = "quality", direction = "higher_is_better",
	})

	local unit_files = by_file(produced.unit)
	local rows = {}
	for file, blackbox_pct in pairs(by_file(produced.blackbox)) do
		local unit_pct = unit_files[file] or 0
		if blackbox_pct - unit_pct >= 40 then
			rows[#rows + 1] = { file = file, delta = blackbox_pct - unit_pct, bb = blackbox_pct, u = unit_pct }
		end
	end
	table.sort(rows, function(a, b) return a.delta > b.delta end)
	for i = 1, math.min(#rows, 10) do
		local row = rows[i]
		print(string.format("  unit-owed  %-58s black-box %5.1f%% · unit %5.1f%%",
			row.file:gsub(prova.root .. "/", ""), row.bb, row.u))
	end
end)
