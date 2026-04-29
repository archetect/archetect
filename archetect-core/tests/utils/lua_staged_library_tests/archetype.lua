-- The catalog entry `test-lib` was declared with library: true, so its
-- lib/ directory was eagerly staged at archetype load. We can require()
-- under the consumer-chosen namespace.
local shouter = require("test-lib.shouter")

output.print(shouter.shout("hello"))

-- archetype.mount_key() returns the catalog map-key when called from
-- inside a staged library, and nil from the parent's own script.
output.print("from-library: " .. tostring(shouter.my_mount_key()))
output.print("from-library is_library: " .. tostring(shouter.am_i_a_library()))
output.print("from-parent: " .. tostring(archetype.mount_key()))
output.print("from-parent is_standalone: " .. tostring(archetype.is_standalone()))

-- include_path() called from the library auto-prefixes with the
-- library's mount key. Called from the parent, returns the path
-- unchanged.
output.print("lib-include: " .. shouter.publish_include("foo.atl"))
output.print("parent-include: " .. archetype.include_path("foo.atl"))
