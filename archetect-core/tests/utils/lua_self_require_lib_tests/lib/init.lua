-- Exposed to the archetype's own shim as `require("lib")` thanks to
-- the `<root>/?/init.lua` entry in package.path. See
-- docs/plans/self-requirable-lib.md.
local M = {}

function M.hello()
    return "hi from self-lib"
end

return M
