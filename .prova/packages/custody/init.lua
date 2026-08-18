--- Report custody, when the runner can take it.
---
--- A deputed conduct hands back three things: cases (the ledger adopts them),
--- measurements (the ratchets hold them), and its own artifact. The artifact is
--- the one that gets dropped: it lives under `target/`, which is swept, so a red
--- floor can refuse a regression while being unable to show what moved.
--- `report.publish` is prova's answer — it copies the artifact into
--- `.prova/var/reports/` and makes it addressable as `prova reports <name>`.
---
--- It is also NEWER THAN THE NEWEST RELEASE. prova v0.24.0 is what CI installs
--- and its `report` global is nil; a dev build from the prova tree has it. Called
--- unconditionally, the whole `ut` lane died on the runner with "attempt to index
--- a nil value (global 'report')" — a toolchain difference reported as a test
--- failure, which is the least useful shape a failure can have.
---
--- So custody degrades: taken where the runner supports it, announced where it
--- cannot be. Never silent — a report that quietly stopped being kept reads
--- exactly like one that was never worth keeping. Delete this shim and call
--- `report.publish` directly once the pinned prova in .github/workflows/ has it.
local M = {}

function M.publish(opts)
	if type(report) ~= "table" or type(report.publish) ~= "function" then
		print(string.format(
			"  (prova %s has no report custody — '%s' not kept: %s)",
			prova.version, opts.name, opts.summary))
		return false
	end
	report.publish(opts)
	return true
end

return M
