-- Library helper. Mounted under <staging>/lib/test-lib/shouter.lua so the
-- consumer can `require("test-lib.shouter")` even though the library's
-- physical location is somewhere else on disk.
local M = {}

function M.shout(text)
    return text:upper() .. "!"
end

return M
