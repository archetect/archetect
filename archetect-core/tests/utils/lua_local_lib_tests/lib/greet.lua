-- Local helper module discovered automatically because lib/ is on
-- package.path. No manifest declaration needed — Phase 1 commit 3
-- of the catalog-driven dependencies plan.
local M = {}

function M.hello(name)
    return "hello, " .. name
end

return M
