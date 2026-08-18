--- Quality gate: no Rust source file grows without bound.
---
--- A 2,000-line file is where bugs hide and where an agent loses the thread —
--- it is the single cheapest structural signal a repo has, and the only one in
--- this directory that costs nothing to check. So unlike its siblings it
--- carries NO switch: it runs in the default `prova` loop.
---
--- Posture: the COUNT of oversized files is ratcheted, and the offenders are
--- named on every run. `modules.rs` is the standing debt; at a baseline of one,
--- any new giant is a 1 → 2 regression and red immediately.

local LIMIT = 1500

-- Roots come from cargo, never a hardcoded list: a hardcoded list goes silently
-- stale the day a crate is added, and the gate quietly stops watching it.
local roots = prova.fixture("workspace-src-roots", Scope.File, function()
	local out = shell.run({ "cargo", "metadata", "--no-deps", "--format-version", "1" },
		{ cwd = prova.root, timeout = "120s" })
	local meta = json.decode(out.stdout or "{}")
	local dirs = {}
	for _, package in ipairs(meta.packages or {}) do
		local dir = (package.manifest_path or ""):match("^(.*)/Cargo%.toml$")
		if dir and fs.exists(dir .. "/src") then
			dirs[#dirs + 1] = dir .. "/src"
		end
	end
	table.sort(dirs)
	return dirs
end)

-- `wc -l` semantics — count newlines, so the numbers match what a person gets
-- from the shell and can act on directly.
local function line_count(path)
	local _, lines = fs.read(path):gsub("\n", "")
	return lines
end

-- fs.glob's base is a concrete dir: "*.rs" catches files directly under it and
-- "**/*.rs" the nested ones. Globbing both and de-duping is robust regardless of
-- whether "**" also matches depth zero.
local function source_files(dirs)
	local seen, out = {}, {}
	for _, dir in ipairs(dirs) do
		for _, pattern in ipairs({ "*.rs", "**/*.rs" }) do
			for _, path in ipairs(fs.glob(dir, pattern)) do
				if not seen[path] then
					seen[path] = true
					out[#out + 1] = path
				end
			end
		end
	end
	return out
end

prova.test("oversized Rust source files do not multiply past the baseline", {
	proves = "size is the one debt signal that needs no compiler and no judgement call. "
		.. "Ratcheting the COUNT grandfathers what is already here — splitting modules.rs "
		.. "is its own change — while making the next 2,000-line file impossible to add "
		.. "without someone deciding to.",
}, function(t)
	local files = source_files(t:use(roots))
	-- Vacuity guard: a broken root discovery finds no files, and "zero oversized"
	-- would read as a triumph.
	t:expect(#files > 50, "the scan found the workspace's sources (" .. #files .. " files)")
		:equals(true)

	local oversized = {}
	for _, path in ipairs(files) do
		local lines = line_count(path)
		if lines > LIMIT then
			oversized[#oversized + 1] = { path = path, lines = lines }
		end
	end
	table.sort(oversized, function(a, b) return a.lines > b.lines end)
	for _, row in ipairs(oversized) do
		print(string.format("  %5d  %s", row.lines, row.path:gsub(prova.root .. "/", "")))
	end

	measure.ratchet(t, "rust.files.oversized", #oversized, { set = "quality" })
end)
