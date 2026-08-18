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

local custody = require("custody")

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

--- Per-file line percent from a full report.
local function by_file(report_)
	local out = {}
	for _, file in ipairs(report_.data[1].files or {}) do
		out[file.filename] = file.summary.lines.percent
	end
	return out
end

--- What coverage is measured ABOUT: this repo's shipping code.
---
--- Two exclusions, each for its own reason, both structural rather than tied to
--- where a particular machine keeps its build dir:
---
---   build-script output — `<anything>/build/<pkg>-<hash>/out/…` is cargo's own
---   convention. `archetect-core/build.rs` compiles the proto with prost, and
---   407 lines of its codegen is not archetect's coverage to earn or lose. It
---   also DIVERGED the layers: the subject build linked it, the nextest build
---   used a different build-script hash, and the two reports disagreed on the
---   denominator by exactly one file until this landed.
---
---   xtask — build automation. It runs on a developer's machine to produce
---   artifacts, it is never in a release, and no user can reach it. Measuring it
---   puts lines a test suite has no business exercising into the denominator of
---   every layer, which is not a coverage gap but a category error.
---
--- `.*` between `build` and `out` on purpose: the classic layout is
--- `build/<pkg>-<hash>/out/`, and cargo's newer build-dir splits it into
--- `build/<pkg>/<hash>/out/`. A pattern pinned to one segment matched nothing
--- under the other and let 407 lines of codegen back into the denominator.
local IGNORE_FILES = "(/build/.*/out/)|(^|/)xtask/"

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
	local out = shell.run(
		{ "cargo", "llvm-cov", "report", "--json", "--ignore-filename-regex", IGNORE_FILES },
		{ cwd = prova.root, env = llvm_cov_env(), timeout = "600s" })
	-- Say what broke. An unchecked failure here hands `json.decode` an empty
	-- string, and the conduct dies on "EOF while parsing" three frames away from
	-- the command that actually failed.
	if out.code ~= 0 or (out.stdout or "") == "" then
		error(string.format("`cargo llvm-cov report` failed (exit %s):\n%s",
			tostring(out.code), (out.stderr or "") .. (out.stdout or "")))
	end
	return json.decode(out.stdout)
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

--- Sweep LLVM's fallback profraws out of the source tree.
---
--- An instrumented process whose `LLVM_PROFILE_FILE` does not reach it writes
--- `default_<sig>_0_<pid>.profraw` into its own CWD — LLVM's default, not a path
--- this conduct controls. Measured: 1,385 of them across the repo root and
--- `archetect-aml/`, and they were staged into a commit before `.gitignore`
--- learned about them.
---
--- DELETED rather than swept into the scan, deliberately. The data is mergeable,
--- but nothing says which layer produced it — a unit-test binary's profraw folded
--- into the black-box layer would inflate exactly the number this suite exists to
--- keep honest. A little lost coverage beats coverage attributed to the wrong
--- layer, and the `suite-*.profraw` floor below already proves the black-box
--- layer measured the subject.
local function sweep_strays()
	local roots = { prova.root, prova.root .. "/xtask" }
	for _, dir in ipairs(fs.glob(prova.root, "archetect-*")) do
		roots[#roots + 1] = dir
	end
	local swept = 0
	for _, dir in ipairs(roots) do
		if fs.exists(dir) then
			for _, path in ipairs(fs.glob(dir, "default_*.profraw")) do
				fs.remove_all(path)
				swept = swept + 1
			end
		end
	end
	return swept
end

local conduct = prova.fixture("layered-coverage", Scope.File, function()
	fs.remove_all(UNIT_STAGE)
	sweep_strays()

	-- EVERY CONDUCT STARTS CLEAN, and the incremental path is gone on purpose.
	--
	-- `report` derives its denominator from every instrumented object it can
	-- scan, and cargo does not evict the previous build's objects from `deps/`.
	-- A second conduct therefore scans two generations of the same crates and
	-- adds their line counts together. Measured live, twice:
	--
	--   * deleting the `group` option — code REMOVED — moved the denominator from
	--     20,695 lines to 22,038 and dropped all three layers ~4pp. Every floor
	--     tripped, and it looked exactly like lost coverage.
	--   * a repeat conduct on unchanged sources reported the unit layer over
	--     20,682 lines and the black-box layer over 21,089, because relinking the
	--     subject left the prior binary's objects in the scan.
	--
	-- A digest-stamped wipe fixed the first and not the second: sources are not
	-- the only input to what ends up in `deps/`. Since a percent whose
	-- denominator moves on its own is not a metric, the dir goes. That costs a
	-- clean instrumented rebuild per conduct — minutes, on a lane that runs
	-- nightly and on demand, which is the right place to spend them.
	--
	-- The wipe also settles a second failure mode it used to take a stamp to
	-- avoid: a half-migrated llvm-cov layout breaking the next conduct's rustc
	-- probe with a wall of identical errors.
	fs.remove_all(COV_DIR)
	fs.mkdir(COV_DIR)

	-- ORDER MATTERS, and both halves of it were learned the hard way.
	--
	-- The SUBJECT is built first so its object is registered before anything
	-- reports. `report` derives its denominator from every instrumented object it
	-- can scan, so a layer reported before the binary exists answers over a
	-- smaller population than one reported after — a ten-point swing between a
	-- cold conduct and a warm one with no code change anywhere.
	--
	-- The UNIT layer then runs before the black-box layer for the same reason in
	-- reverse: nextest's test binaries join the object set too. Once both are
	-- present, all three layers count the same lines, which is the whole point of
	-- measuring three — unit 61% and black-box 27% only add up to a merged 72% if
	-- they share a denominator, and the unit-owed worklist at the bottom subtracts
	-- one layer's per-file percent from the other's.
	--
	-- The alternative (hide the test binaries while the black-box layer reports,
	-- for a smaller and arguably purer denominator) is deliberately not taken:
	-- comparability across layers is worth more here than layer purity.

	-- The instrumented subject, built BY cargo-llvm-cov rather than by a bare
	-- `cargo build --target-dir`.
	--
	-- This is not a style choice. `report` attributes a profraw's counters using
	-- the object list llvm-cov itself recorded, and a binary cargo built behind
	-- its back is not on that list — so its counters are silently dropped.
	-- Measured: the black-box layer read 2.46%, covering only the dependency
	-- rlibs nextest had registered (git-cache, api, terminal-io) and NOTHING from
	-- archetect-core or archetect-bin, which is where a CLI's work happens. No
	-- error, no warning, just a number 25 points low.
	--
	-- `run --bin` is the way to make llvm-cov do the build and register the
	-- object. `--version` is a throwaway argument — the binary has to be invoked
	-- for `run` to exist, and the profraw it drops is purged below so the layer
	-- measures the suite and not this.
	local build = shell.run(
		{ "cargo", "llvm-cov", "run", "--no-report", "--bin", "archetect", "--", "--version" },
		{ cwd = prova.root, env = llvm_cov_env(), timeout = "1800s", merge_stderr = true })
	if build.code ~= 0 then
		return { error = "instrumented build failed:\n" .. (build.stdout or "") }
	end
	purge(COV_DIR, "*.profraw")

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
	-- The legs above are what drop strays; leaving them would litter the tree the
	-- moment someone runs this lane and forgets what wrote the files.
	sweep_strays()

	-- Custody. The conduct has produced the answer to "which lines"; without this
	-- it is discarded with `target/`, and a red floor can refuse a regression
	-- while being unable to show what moved. Two forms of one fact: llvm-cov's
	-- own HTML for a person, the merged JSON for an agent (and for diffing runs).
	local html_dir = prova.root .. "/target/coverage-html"
	fs.remove_all(html_dir)
	local html = shell.run(
		{ "cargo", "llvm-cov", "report", "--html", "--output-dir", html_dir,
		  "--ignore-filename-regex", IGNORE_FILES },
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

	custody.publish {
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

	-- The exclusion is a regex in a shell argument: a typo silently measures
	-- everything again and the floors drift for a reason nobody can see. Assert
	-- the denominator itself rather than trusting the flag went through.
	for _, layer in ipairs({ produced.unit, produced.blackbox, produced.merged }) do
		local strays = {}
		for file in pairs(by_file(layer)) do
			if file:find("/out/", 1, true) or file:find("/xtask/", 1, true) then
				strays[#strays + 1] = file
			end
		end
		t:expect(strays, "generated code and build automation stay out of the denominator")
			:has_length(0)
	end

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
