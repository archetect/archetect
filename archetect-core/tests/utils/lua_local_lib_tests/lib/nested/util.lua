-- Nested module: require("nested.util") should find this file at
-- <root>/lib/nested/util.lua via the standard package.path patterns.
local M = {}

function M.shout(text)
    return text:upper() .. "!"
end

return M
