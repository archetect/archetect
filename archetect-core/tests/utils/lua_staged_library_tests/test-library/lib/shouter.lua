-- Library helper. Mounted under <staging>/lib/test-lib/shouter.lua so the
-- consumer can `require("test-lib.shouter")` even though the library's
-- physical location is somewhere else on disk.
local M = {}

function M.shout(text)
    return text:upper() .. "!"
end

-- Introspect the mount key from inside library code. archetype.mount_key()
-- walks the Lua call stack to identify the staged library the calling
-- chunk lives in — so this returns the consumer-chosen catalog map-key
-- (here, "test-lib"), not the library's physical name.
function M.my_mount_key()
    return archetype.mount_key()
end

function M.am_i_a_library()
    return archetype.is_library()
end

function M.publish_include(rel)
    -- include_path() is sugar over mount_key — when called from inside
    -- a staged library, it auto-prefixes the relative path with the
    -- library's mount key. The library author never has to know what
    -- key the parent chose.
    return archetype.include_path(rel)
end

return M
