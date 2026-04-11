-- The catalog entry `test-lib` was declared with library: true, so its
-- lib/ directory was eagerly staged at archetype load. We can require()
-- under the consumer-chosen namespace.
local shouter = require("test-lib.shouter")

output.print(shouter.shout("hello"))
