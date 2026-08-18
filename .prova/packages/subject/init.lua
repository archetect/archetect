--- The system under proof: the archetect binary itself.
---
--- Every suite in this tree drives the SHIPPED CLI, never library internals, so
--- every suite needs the same thing — one build, one path. Declaring it once here
--- rather than copy-pasting a `Scope.File` fixture per file buys two things:
---
---   * `Scope.Run` — the workspace is built at most once per run, not once per
---     proof file that happens to need it.
---   * an override hook — the coverage lane builds an INSTRUMENTED archetect into
---     its own target dir and points the suite at it, which is the only way a
---     black-box suite can contribute to a coverage number at all.
---
--- The override is an env var rather than a switch because the process that sets
--- it is a DIFFERENT prova run: the coverage conduct shells out to a plain
--- `prova`, and a switch cannot cross a process boundary.
local M = {}

M.bin = prova.fixture("archetect-bin", Scope.Run, function(ctx)
	local instrumented = os.getenv("ARCHETECT_SUBJECT_BIN")
	if instrumented and instrumented ~= "" then
		-- Named by the conductor; building over it would discard the
		-- instrumentation that is the whole reason it was named.
		return instrumented
	end
	shell.run("cargo build -p archetect", { cwd = prova.root, timeout = "600s", check = true })
	return prova.root .. "/target/debug/archetect"
end)

return M
